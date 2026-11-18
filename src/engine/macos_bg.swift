//
// macos_bg.swift - Native macOS Live & Static Wallpaper Engine for Aura
// Ultra-low CPU (VideoToolbox / Metal / AVFoundation) & Seamless MPV JSON-IPC compatibility
//

import Cocoa
import AVFoundation
import CoreGraphics
import CoreMedia
import CryptoKit
import Darwin
import VideoToolbox

private final class TemporalWallpaperWriter {
    private let outputURL: URL
    private let lock = NSLock()
    private var writer: AVAssetWriter?
    private var input: AVAssetWriterInput?
    private(set) var error: Error?

    init(outputURL: URL) {
        self.outputURL = outputURL
    }

    func append(_ sampleBuffer: CMSampleBuffer) {
        lock.lock()
        defer { lock.unlock() }
        guard error == nil else { return }

        do {
            if writer == nil {
                guard let format = CMSampleBufferGetFormatDescription(sampleBuffer) else {
                    throw NSError(domain: "AuraWallpaper", code: 10, userInfo: [NSLocalizedDescriptionKey: "HEVC encoder returned no format description"])
                }
                let assetWriter = try AVAssetWriter(outputURL: outputURL, fileType: .mov)
                let writerInput = AVAssetWriterInput(mediaType: .video, outputSettings: nil, sourceFormatHint: format)
                writerInput.expectsMediaDataInRealTime = false
                guard assetWriter.canAdd(writerInput) else {
                    throw NSError(domain: "AuraWallpaper", code: 11, userInfo: [NSLocalizedDescriptionKey: "Cannot add the HEVC stream to the output file"])
                }
                assetWriter.add(writerInput)
                guard assetWriter.startWriting() else {
                    throw assetWriter.error ?? NSError(domain: "AuraWallpaper", code: 12, userInfo: [NSLocalizedDescriptionKey: "Could not start writing the HEVC stream"])
                }
                assetWriter.startSession(atSourceTime: CMSampleBufferGetPresentationTimeStamp(sampleBuffer))
                writer = assetWriter
                input = writerInput
            }

            guard let writer, let input else { return }
            while !input.isReadyForMoreMediaData {
                if writer.status == .failed {
                    throw writer.error ?? NSError(domain: "AuraWallpaper", code: 13, userInfo: [NSLocalizedDescriptionKey: "Writing the HEVC stream failed"])
                }
                usleep(1_000)
            }
            guard input.append(sampleBuffer) else {
                throw writer.error ?? NSError(domain: "AuraWallpaper", code: 14, userInfo: [NSLocalizedDescriptionKey: "Could not append a frame to the HEVC stream"])
            }
        } catch {
            self.error = error
        }
    }

    func finish() throws {
        lock.lock()
        if let error {
            lock.unlock()
            throw error
        }
        guard let writer, let input else {
            lock.unlock()
            throw NSError(domain: "AuraWallpaper", code: 15, userInfo: [NSLocalizedDescriptionKey: "The HEVC encoder produced no frames"])
        }
        input.markAsFinished()
        lock.unlock()

        let completed = DispatchSemaphore(value: 0)
        writer.finishWriting { completed.signal() }
        completed.wait()
        guard writer.status == .completed else {
            throw writer.error ?? NSError(domain: "AuraWallpaper", code: 16, userInfo: [NSLocalizedDescriptionKey: "Could not finish the HEVC video"])
        }
    }
}

private let temporalWallpaperOutputCallback: VTCompressionOutputCallback = { refCon, _, status, _, sampleBuffer in
    guard status == noErr, let refCon, let sampleBuffer else { return }
    Unmanaged<TemporalWallpaperWriter>.fromOpaque(refCon).takeUnretainedValue().append(sampleBuffer)
}

class WallpaperWindow: NSWindow {
    init(screen: NSScreen) {
        super.init(
            contentRect: screen.frame,
            styleMask: [.borderless],
            backing: .buffered,
            defer: false
        )
        self.isOpaque = true
        self.hasShadow = false
        self.backgroundColor = .black
        self.level = NSWindow.Level(rawValue: Int(CGWindowLevelForKey(.desktopWindow)) - 1)
        self.collectionBehavior = [.canJoinAllSpaces, .stationary, .ignoresCycle]
        self.ignoresMouseEvents = true
        self.setFrame(screen.frame, display: true)
    }

    override var canBecomeKey: Bool { false }
    override var canBecomeMain: Bool { false }
}

class WallpaperPlayer: NSObject {
    var windows: [WallpaperWindow] = []
    var targetScreens: [NSScreen] = []
    private let fileOperationLock = NSLock()
    private let lockscreenSyncLock = NSLock()
    var playerViews: [NSView] = []
    var queuePlayer: AVQueuePlayer?
    var playerLooper: AVPlayerLooper?
    var playerLayers: [AVPlayerLayer] = []
    var isImage: Bool = false
    var isPaused: Bool = false
    var currentPath: String = ""
    var volume: Float = 1.0
    var isMuted: Bool = true
    var scalingMode: String = "fit" // "fit", "fill", "stretch"

    func setupScreens(outputName: String) -> [NSScreen] {
        if outputName == "*" || outputName == "all" || outputName.isEmpty {
            return NSScreen.screens
        }
        var seen = Set<String>()
        for (i, screen) in NSScreen.screens.enumerated() {
            var name = screen.localizedName
            if seen.contains(name) {
                name = "\(name) (\(i + 1))"
            }
            seen.insert(name)
            if name == outputName {
                return [screen]
            }
        }
        // A disconnected/unknown output must never change a different display.
        return []
    }

    func applyScaling() {
        let gravity: AVLayerVideoGravity
        switch scalingMode {
        case "fill":
            gravity = .resizeAspectFill
        case "stretch":
            gravity = .resize
        default:
            gravity = .resizeAspect
        }
        for layer in playerLayers {
            layer.videoGravity = gravity
        }
    }

    func loadMedia(path: String) {
        currentPath = path
        let fileUrl = URL(fileURLWithPath: path)
        let ext = fileUrl.pathExtension.lowercased()
        let imageExtensions = ["jpg", "jpeg", "png", "webp", "gif", "bmp", "tiff"]
        isImage = imageExtensions.contains(ext)
        if isImage {
            syncDesktopImage(from: fileUrl)
        }

        // Clear existing player
        playerLooper = nil
        queuePlayer?.pause()
        queuePlayer = nil
        playerLayers.removeAll()

        for window in windows {
            let contentView = window.contentView ?? NSView(frame: window.frame)
            window.contentView = contentView
            contentView.subviews.removeAll()
            contentView.layer?.sublayers?.removeAll()
            contentView.wantsLayer = true
            contentView.layer?.backgroundColor = NSColor.black.cgColor

            if isImage {
                if let image = NSImage(contentsOf: fileUrl) {
                    let imageView = NSImageView(frame: contentView.bounds)
                    imageView.autoresizingMask = [.width, .height]
                    imageView.image = image
                    switch scalingMode {
                    case "fill":
                        imageView.imageScaling = .scaleAxesIndependently
                    case "stretch":
                        imageView.imageScaling = .scaleAxesIndependently
                    default:
                        imageView.imageScaling = .scaleProportionallyUpOrDown
                    }
                    contentView.addSubview(imageView)
                }
            } else {
                let asset = AVURLAsset(url: fileUrl)
                let item = AVPlayerItem(asset: asset)
                if queuePlayer == nil {
                    let player = AVQueuePlayer(playerItem: item)
                    player.isMuted = isMuted
                    player.volume = volume
                    playerLooper = AVPlayerLooper(player: player, templateItem: item)
                    queuePlayer = player
                }

                let layer = AVPlayerLayer(player: queuePlayer)
                layer.frame = contentView.bounds
                layer.autoresizingMask = [.layerWidthSizable, .layerHeightSizable]
                contentView.layer?.addSublayer(layer)
                playerLayers.append(layer)
            }

            window.orderFront(nil)
        }

        applyScaling()

        if !isImage && !isPaused {
            queuePlayer?.play()
        }
    }

    func displayUUID(for screen: NSScreen) -> String? {
        guard let number = screen.deviceDescription[NSDeviceDescriptionKey("NSScreenNumber")] as? NSNumber,
              let unmanagedUUID = CGDisplayCreateUUIDFromDisplayID(number.uint32Value) else {
            return nil
        }
        return CFUUIDCreateString(kCFAllocatorDefault, unmanagedUUID.takeRetainedValue()) as String
    }

    private func syncDesktopImage(from sourceURL: URL) {
        let fileManager = FileManager.default
        guard let supportDirectory = fileManager.urls(for: .applicationSupportDirectory, in: .userDomainMask).first else {
            return
        }

        let wallpaperDirectory = supportDirectory.appendingPathComponent("Aura/LoginWallpapers", isDirectory: true)
        do {
            try fileManager.createDirectory(at: wallpaperDirectory, withIntermediateDirectories: true)
        } catch {
            fputs("[aura-bg] Could not create login wallpaper directory: \(error)\n", stderr)
            return
        }

        guard let image = NSImage(contentsOf: sourceURL) else { return }
        var imageRect = CGRect(origin: .zero, size: image.size)
        let cgImage = image.cgImage(forProposedRect: &imageRect, context: nil, hints: nil)

        guard let cgImage,
              let imageData = NSBitmapImageRep(cgImage: cgImage).representation(
                using: .jpeg,
                properties: [.compressionFactor: 0.92]
              ) else {
            fputs("[aura-bg] Could not encode desktop image: \(sourceURL.path)\n", stderr)
            return
        }

        let workspace = NSWorkspace.shared
        for (index, screen) in targetScreens.enumerated() {
            let screenNumber = screen.deviceDescription[NSDeviceDescriptionKey("NSScreenNumber")] as? NSNumber
            let screenID = screenNumber?.stringValue ?? "display-\(index)"
            let wallpaperURL = wallpaperDirectory.appendingPathComponent("screen-\(screenID).jpg")

            do {
                try imageData.write(to: wallpaperURL, options: .atomic)

                var options = workspace.desktopImageOptions(for: screen) ?? [:]
                let imageScaling: NSImageScaling = scalingMode == "stretch"
                    ? .scaleAxesIndependently
                    : .scaleProportionallyUpOrDown
                options[.imageScaling] = NSNumber(value: imageScaling.rawValue)
                options[.allowClipping] = NSNumber(value: scalingMode == "fill")
                try workspace.setDesktopImageURL(wallpaperURL, for: screen, options: options)
            } catch {
                fputs("[aura-bg] Could not set desktop image for \(screen.localizedName): \(error)\n", stderr)
            }
        }
    }

    private func withFileLock<T>(at url: URL, _ body: () throws -> T) throws -> T {
        fileOperationLock.lock()
        defer { fileOperationLock.unlock() }
        let descriptor = Darwin.open(url.path, O_CREAT | O_RDWR, mode_t(0o600))
        guard descriptor >= 0 else {
            throw NSError(domain: NSPOSIXErrorDomain, code: Int(errno))
        }
        guard Darwin.lockf(descriptor, F_LOCK, 0) == 0 else {
            let error = NSError(domain: NSPOSIXErrorDomain, code: Int(errno))
            Darwin.close(descriptor)
            throw error
        }
        defer {
            _ = Darwin.lockf(descriptor, F_ULOCK, 0)
            Darwin.close(descriptor)
        }
        return try body()
    }

    private func encodeTemporalWallpaper(
        from sourceURL: URL,
        to outputURL: URL,
        progress: ((Double) -> Void)? = nil
    ) async throws {
        let asset = AVURLAsset(url: sourceURL)
        guard let track = try await asset.loadTracks(withMediaType: .video).first else {
            throw NSError(domain: "AuraWallpaper", code: 20, userInfo: [NSLocalizedDescriptionKey: "The selected file has no video track"])
        }
        let naturalSize = try await track.load(.naturalSize)
        let width = Int(abs(naturalSize.width).rounded()) & ~1
        let height = Int(abs(naturalSize.height).rounded()) & ~1
        let nominalFrameRate = try await track.load(.nominalFrameRate)
        let sourceFrameRate = Double(nominalFrameRate >= 1 ? nominalFrameRate : 30)
        // Keep the source cadence; VideoToolbox adds the temporal sub-layers used by Aerial.
        let aerialFrameRate = sourceFrameRate
        let estimatedDataRate = try await track.load(.estimatedDataRate)
        let bitRate = max(2_000_000, min(16_000_000, Int(estimatedDataRate)))
        let duration = try await asset.load(.duration)
        let durationSeconds = CMTimeGetSeconds(duration)
        guard width > 0, height > 0, duration.isNumeric, durationSeconds.isFinite, durationSeconds > 0 else {
            throw NSError(domain: "AuraWallpaper", code: 21, userInfo: [NSLocalizedDescriptionKey: "The selected video has invalid dimensions or duration"])
        }

        // Aerials don't loop short custom clips. Keep the login-screen video playing for about five minutes.
        let loopCount = max(1, min(300, Int(ceil(300 / durationSeconds))))
        let estimatedSourceFrames = max(1, Int((durationSeconds * sourceFrameRate).rounded(.up)) * loopCount)
        let output = TemporalWallpaperWriter(outputURL: outputURL)
        var compressionSession: VTCompressionSession?
        let createStatus = VTCompressionSessionCreate(
            allocator: kCFAllocatorDefault,
            width: Int32(width),
            height: Int32(height),
            codecType: kCMVideoCodecType_HEVC,
            encoderSpecification: [kVTVideoEncoderSpecification_EnableHardwareAcceleratedVideoEncoder: true] as CFDictionary,
            imageBufferAttributes: nil,
            compressedDataAllocator: nil,
            outputCallback: temporalWallpaperOutputCallback,
            refcon: Unmanaged.passUnretained(output).toOpaque(),
            compressionSessionOut: &compressionSession
        )
        guard createStatus == noErr, let session = compressionSession else {
            throw NSError(domain: "AuraWallpaper", code: Int(createStatus), userInfo: [NSLocalizedDescriptionKey: "Could not create the HEVC hardware encoder"])
        }
        defer { VTCompressionSessionInvalidate(session) }

        func setEncoderProperty(_ key: CFString, _ value: CFTypeRef) throws {
            let status = VTSessionSetProperty(session, key: key, value: value)
            guard status == noErr else {
                throw NSError(domain: "AuraWallpaper", code: Int(status), userInfo: [NSLocalizedDescriptionKey: "The HEVC encoder does not support a required temporal-layer setting"])
            }
        }
        try setEncoderProperty(kVTCompressionPropertyKey_RealTime, kCFBooleanFalse)
        try setEncoderProperty(kVTCompressionPropertyKey_ProfileLevel, kVTProfileLevel_HEVC_Main10_AutoLevel)
        try setEncoderProperty(kVTCompressionPropertyKey_AllowFrameReordering, kCFBooleanTrue)
        try setEncoderProperty(kVTCompressionPropertyKey_AllowTemporalCompression, kCFBooleanTrue)
        try setEncoderProperty(kVTCompressionPropertyKey_BaseLayerFrameRate, NSNumber(value: aerialFrameRate / 2.0))
        try setEncoderProperty(kVTCompressionPropertyKey_ExpectedFrameRate, NSNumber(value: aerialFrameRate))
        try setEncoderProperty(kVTCompressionPropertyKey_AverageBitRate, NSNumber(value: bitRate))
        try setEncoderProperty(kVTCompressionPropertyKey_MaxKeyFrameInterval, NSNumber(value: Int(aerialFrameRate * 5)))
        try setEncoderProperty(kVTCompressionPropertyKey_ColorPrimaries, kCVImageBufferColorPrimaries_ITU_R_709_2)
        try setEncoderProperty(kVTCompressionPropertyKey_TransferFunction, kCVImageBufferTransferFunction_ITU_R_709_2)
        try setEncoderProperty(kVTCompressionPropertyKey_YCbCrMatrix, kCVImageBufferYCbCrMatrix_ITU_R_709_2)
        let prepareStatus = VTCompressionSessionPrepareToEncodeFrames(session)
        guard prepareStatus == noErr else {
            throw NSError(domain: "AuraWallpaper", code: Int(prepareStatus), userInfo: [NSLocalizedDescriptionKey: "Could not prepare the HEVC encoder"])
        }

        var processedSourceFrames = 0
        var lastReportedFrame = 0
        let progressFrameStep = max(1, Int(min(sourceFrameRate, 480) / 2))
        for loop in 0..<loopCount {
            let reader = try AVAssetReader(asset: asset)
            let readerOutput = AVAssetReaderTrackOutput(track: track, outputSettings: [
                kCVPixelBufferPixelFormatTypeKey as String: kCVPixelFormatType_420YpCbCr10BiPlanarVideoRange
            ])
            readerOutput.alwaysCopiesSampleData = false
            guard reader.canAdd(readerOutput) else {
                throw NSError(domain: "AuraWallpaper", code: 22, userInfo: [NSLocalizedDescriptionKey: "Could not read the selected video for HEVC encoding"])
            }
            reader.add(readerOutput)
            guard reader.startReading() else {
                throw reader.error ?? NSError(domain: "AuraWallpaper", code: 23, userInfo: [NSLocalizedDescriptionKey: "Could not start reading the selected video"])
            }

            let loopOffset = CMTimeMultiply(duration, multiplier: Int32(loop))
            while let sampleBuffer = readerOutput.copyNextSampleBuffer() {
                guard let imageBuffer = CMSampleBufferGetImageBuffer(sampleBuffer) else { continue }
                processedSourceFrames += 1
                if processedSourceFrames - lastReportedFrame >= progressFrameStep {
                    lastReportedFrame = processedSourceFrames
                    progress?(min(0.98, Double(processedSourceFrames) / Double(estimatedSourceFrames) * 0.98))
                }
                let presentationTime = CMTimeAdd(CMSampleBufferGetPresentationTimeStamp(sampleBuffer), loopOffset)
                var frameDuration = CMSampleBufferGetDuration(sampleBuffer)
                if !frameDuration.isNumeric || frameDuration.value <= 0 {
                    frameDuration = CMTime(value: 1, timescale: Int32(sourceFrameRate.rounded()))
                }
                let encodeStatus = VTCompressionSessionEncodeFrame(
                    session,
                    imageBuffer: imageBuffer,
                    presentationTimeStamp: presentationTime,
                    duration: frameDuration,
                    frameProperties: nil,
                    sourceFrameRefcon: nil,
                    infoFlagsOut: nil
                )
                guard encodeStatus == noErr else {
                    throw NSError(domain: "AuraWallpaper", code: Int(encodeStatus), userInfo: [NSLocalizedDescriptionKey: "The HEVC encoder failed while processing a frame"])
                }
            }
            guard reader.status == .completed else {
                throw reader.error ?? NSError(domain: "AuraWallpaper", code: 24, userInfo: [NSLocalizedDescriptionKey: "Reading the selected video stopped before completion"])
            }
        }

        let finishStatus = VTCompressionSessionCompleteFrames(session, untilPresentationTimeStamp: .invalid)
        guard finishStatus == noErr else {
            throw NSError(domain: "AuraWallpaper", code: Int(finishStatus), userInfo: [NSLocalizedDescriptionKey: "Could not finish the HEVC temporal layers"])
        }
        try output.finish()

        let encodedAsset = AVURLAsset(url: outputURL)
        guard let encodedTrack = try await encodedAsset.loadTracks(withMediaType: .video).first else {
            throw NSError(domain: "AuraWallpaper", code: 25, userInfo: [NSLocalizedDescriptionKey: "The Aerial video has no readable video track"])
        }
        let encodedFrameRate = try await encodedTrack.load(.nominalFrameRate)
        guard encodedFrameRate > 0 else {
            throw NSError(domain: "AuraWallpaper", code: 26, userInfo: [NSLocalizedDescriptionKey: "The Aerial video did not retain its source frame rate"])
        }
    }

    @discardableResult
    func syncLockScreenVideo(
        from sourceURL: URL,
        displayUUIDs: [String],
        defaultDisplayUUID: String?,
        progress: ((Double) -> Void)? = nil
    ) -> Bool {
        guard ProcessInfo.processInfo.operatingSystemVersion.majorVersion >= 26,
              !displayUUIDs.isEmpty,
              let supportDirectory = FileManager.default.urls(for: .applicationSupportDirectory, in: .userDomainMask).first else {
            return false
        }

        let fileManager = FileManager.default
        let auraDirectory = supportDirectory.appendingPathComponent("Aura/LoginWallpapers", isDirectory: true)
        do {
            try fileManager.createDirectory(at: auraDirectory, withIntermediateDirectories: true)
        } catch {
            fputs("[aura-bg] Could not create lock-screen wallpaper directory: \(error.localizedDescription)\n", stderr)
            return false
        }

        lockscreenSyncLock.lock()
        defer { lockscreenSyncLock.unlock() }
        let queueLockURL = auraDirectory.appendingPathComponent("lockscreen-conversion.lock")
        let queueLockFD = Darwin.open(queueLockURL.path, O_CREAT | O_RDWR, mode_t(0o600))
        guard queueLockFD >= 0 else {
            fputs("[aura-bg] Could not open the lock-screen conversion queue lock.\n", stderr)
            return false
        }
        guard Darwin.lockf(queueLockFD, F_LOCK, 0) == 0 else {
            let error = NSError(domain: NSPOSIXErrorDomain, code: Int(errno))
            Darwin.close(queueLockFD)
            fputs("[aura-bg] Could not lock the lock-screen conversion queue: \(error.localizedDescription)\n", stderr)
            return false
        }
        defer {
            _ = Darwin.lockf(queueLockFD, F_ULOCK, 0)
            Darwin.close(queueLockFD)
        }
        let aerialDirectory = supportDirectory.appendingPathComponent("com.apple.wallpaper/aerials", isDirectory: true)
        let videoDirectory = aerialDirectory.appendingPathComponent("videos", isDirectory: true)
        let manifestURL = aerialDirectory.appendingPathComponent("manifest/entries.json")
        let indexURL = supportDirectory.appendingPathComponent("com.apple.wallpaper/Store/Index.plist")
        let lockURL = auraDirectory.appendingPathComponent("lockscreen.lock")

        do {
            try fileManager.createDirectory(at: auraDirectory, withIntermediateDirectories: true)
            guard fileManager.fileExists(atPath: manifestURL.path),
                  fileManager.fileExists(atPath: indexURL.path) else {
                fputs("[aura-bg] Download an Aerial wallpaper in System Settings to enable lock-screen video.\n", stderr)
                return false
            }
            progress?(0.01)

            let attributes = try fileManager.attributesOfItem(atPath: sourceURL.path)
            let signatureText = "aerial-temporal-hevc-v3\n\(sourceURL.standardizedFileURL.path)\n\(attributes[.size] ?? 0)\n\((attributes[.modificationDate] as? Date)?.timeIntervalSince1970 ?? 0)"
            let signature = SHA256.hash(data: Data(signatureText.utf8)).map { String(format: "%02x", $0) }.joined()
            let convertedDirectory = auraDirectory.appendingPathComponent("Converted", isDirectory: true)
            try fileManager.createDirectory(at: convertedDirectory, withIntermediateDirectories: true)
            // A killed encoder can leave hidden, incomplete MOVs; the queue lock guarantees none are active here.
            for entry in try fileManager.contentsOfDirectory(at: convertedDirectory, includingPropertiesForKeys: [.isRegularFileKey])
            where entry.lastPathComponent.hasPrefix(".") && ["mov", "complete"].contains(entry.pathExtension) {
                try? fileManager.removeItem(at: entry)
            }
            let convertedURL = convertedDirectory.appendingPathComponent("\(signature).mov")
            let conversionLockURL = convertedDirectory.appendingPathComponent("\(signature).lock")

            try withFileLock(at: conversionLockURL) {
                if !fileManager.fileExists(atPath: convertedURL.path) {
                    let tempURL = convertedDirectory.appendingPathComponent(".\(UUID().uuidString).mov")
                    let successURL = tempURL.appendingPathExtension("complete")
                    defer {
                        try? fileManager.removeItem(at: tempURL)
                        try? fileManager.removeItem(at: successURL)
                    }
                    let completed = DispatchSemaphore(value: 0)
                    Task {
                        do {
                            try await self.encodeTemporalWallpaper(from: sourceURL, to: tempURL, progress: progress)
                            try Data().write(to: successURL, options: .atomic)
                        } catch {
                            fputs("[aura-bg] Lock-screen video conversion failed: \(error.localizedDescription)\n", stderr)
                        }
                        completed.signal()
                    }
                    completed.wait()
                    guard fileManager.fileExists(atPath: successURL.path), fileManager.fileExists(atPath: tempURL.path) else {
                        return
                    }
                    try fileManager.moveItem(at: tempURL, to: convertedURL)
                }
            }

            guard fileManager.fileExists(atPath: convertedURL.path) else { return false }
            progress?(0.99)
            var shouldReload = false
            try withFileLock(at: lockURL) {
                let slotsURL = auraDirectory.appendingPathComponent("slots.json")
                var slots: [String: String] = [:]
                if fileManager.fileExists(atPath: slotsURL.path) {
                    let data = try Data(contentsOf: slotsURL)
                    guard let savedSlots = try JSONSerialization.jsonObject(with: data) as? [String: String] else {
                        throw NSError(domain: "AuraWallpaper", code: 1, userInfo: [NSLocalizedDescriptionKey: "Invalid saved lock-screen slot map"])
                    }
                    slots = savedSlots
                }
                guard var manifest = try JSONSerialization.jsonObject(with: Data(contentsOf: manifestURL)) as? [String: Any],
                      var assets = manifest["assets"] as? [[String: Any]],
                      var index = try PropertyListSerialization.propertyList(from: Data(contentsOf: indexURL), options: [], format: nil) as? [String: Any] else {
                    throw NSError(domain: "AuraWallpaper", code: 2, userInfo: [NSLocalizedDescriptionKey: "Invalid macOS wallpaper catalog"])
                }
                // Register our own local assets, as CaveWall does. Borrowing an Apple UUID
                // lets System Settings download its original over our video later.
                let categoryID = "4CF455BA-0E3B-4D58-994A-B9CA6C07F0A1"
                let subcategoryID = "4CF455BA-0E3B-4D58-994A-B9CA6C07F0A2"
                let existingID = assets.first { entry in
                    guard entry["shotID"] as? String == "AURA_LOCAL",
                          let id = entry["id"] as? String, UUID(uuidString: id) != nil else { return false }
                    return (try? String(contentsOf: auraDirectory.appendingPathComponent("\(id).source"), encoding: .utf8)) == signature
                }?["id"] as? String
                let slotID = existingID ?? UUID().uuidString
                let slotURL = videoDirectory.appendingPathComponent("\(slotID).mov")
                let markerURL = auraDirectory.appendingPathComponent("\(slotID).source")
                let thumbnailDirectory = aerialDirectory.appendingPathComponent("thumbnails", isDirectory: true)
                let thumbnailURL = thumbnailDirectory.appendingPathComponent("\(slotID).png")
                try fileManager.createDirectory(at: videoDirectory, withIntermediateDirectories: true)
                try fileManager.createDirectory(at: thumbnailDirectory, withIntermediateDirectories: true)
                let videoChanged = !fileManager.contentsEqual(atPath: slotURL.path, andPath: convertedURL.path)
                if videoChanged {
                    let tempURL = videoDirectory.appendingPathComponent(".\(UUID().uuidString).mov")
                    defer { try? fileManager.removeItem(at: tempURL) }
                    try fileManager.copyItem(at: convertedURL, to: tempURL)
                    guard Darwin.rename(tempURL.path, slotURL.path) == 0 else {
                        throw NSError(domain: NSPOSIXErrorDomain, code: Int(errno))
                    }
                }
                if !fileManager.fileExists(atPath: thumbnailURL.path) {
                    let generator = AVAssetImageGenerator(asset: AVURLAsset(url: convertedURL))
                    generator.appliesPreferredTrackTransform = true
                    generator.maximumSize = CGSize(width: 512, height: 512)
                    let frame = try generator.copyCGImage(at: CMTime(seconds: 1, preferredTimescale: 600), actualTime: nil)
                    guard let png = NSBitmapImageRep(cgImage: frame).representation(using: .png, properties: [:]) else {
                        throw NSError(domain: "AuraWallpaper", code: 3, userInfo: [NSLocalizedDescriptionKey: "Could not create the lock-screen video preview"])
                    }
                    try png.write(to: thumbnailURL, options: .atomic)
                }
                let catalogChanged = existingID == nil
                if catalogChanged {
                    let backupURL = auraDirectory.appendingPathComponent("entries.original.json")
                    if !fileManager.fileExists(atPath: backupURL.path) {
                        try fileManager.copyItem(at: manifestURL, to: backupURL)
                    }
                    let name = sourceURL.deletingPathExtension().lastPathComponent
                    assets.append([
                        "id": slotID, "shotID": "AURA_LOCAL",
                        "localizedNameKey": name, "accessibilityLabel": name,
                        "previewImage": thumbnailURL.absoluteString,
                        "url-4K-SDR-240FPS": convertedURL.absoluteString,
                        "categories": [categoryID], "subcategories": [subcategoryID],
                        "includeInShuffle": false, "showInTopLevel": true,
                        "preferredOrder": 0, "pointsOfInterest": [String: String]()
                    ])
                    var categories = manifest["categories"] as? [[String: Any]] ?? []
                    let section: [String: Any] = [
                        "id": subcategoryID, "localizedNameKey": "Aura",
                        "localizedDescriptionKey": "Videos de Aura",
                        "previewImage": thumbnailURL.absoluteString,
                        "representativeAssetID": slotID, "preferredOrder": 0
                    ]
                    var category = section
                    category["id"] = categoryID
                    category["subcategories"] = [section]
                    categories.removeAll { $0["id"] as? String == categoryID }
                    categories.append(category)
                    manifest["assets"] = assets
                    manifest["categories"] = categories
                    try JSONSerialization.data(withJSONObject: manifest, options: [.prettyPrinted, .sortedKeys])
                        .write(to: manifestURL, options: .atomic)
                    try Data(signature.utf8).write(to: markerURL, options: .atomic)
                }
                let slotsChanged = displayUUIDs.contains { slots[$0] != slotID }
                for displayUUID in displayUUIDs { slots[displayUUID] = slotID }
                if slotsChanged {
                    try JSONSerialization.data(withJSONObject: slots, options: [.prettyPrinted, .sortedKeys])
                        .write(to: slotsURL, options: .atomic)
                }

                var activeSlots: [String: String] = [:]
                let connectedDisplayUUIDs = Set(NSScreen.screens.compactMap { displayUUID(for: $0) })
                for displayUUID in connectedDisplayUUIDs.sorted() {
                    if let slotID = slots[displayUUID] { activeSlots[displayUUID] = slotID }
                }
                let indexChanged = Self.applyAerialSlots(
                    activeSlots,
                    defaultDisplayUUID: defaultDisplayUUID,
                    to: &index
                )
                if slotsChanged || videoChanged || catalogChanged || indexChanged {
                    shouldReload = true
                }
                if indexChanged {
                    let backupURL = auraDirectory.appendingPathComponent("Index.original.plist")
                    if !fileManager.fileExists(atPath: backupURL.path) {
                        try fileManager.copyItem(at: indexURL, to: backupURL)
                    }
                    let data = try PropertyListSerialization.data(fromPropertyList: index, format: .binary, options: 0)
                    try data.write(to: indexURL, options: .atomic)
                }

            }

            if shouldReload || !Self.isProcessRunning("WallpaperAerialsExtension") {
                try Self.restartWallpaperAgent()
            }
            progress?(1)
            return true
        } catch {
            fputs("[aura-bg] Could not apply video to the macOS lock screen: \(error.localizedDescription)\n", stderr)
            return false
        }
    }

    private static func setAerialWallpaper(_ slotID: String, in configuration: inout [String: Any]) -> Bool {
        var changed = false
        if configuration["Type"] as? String != "individual" {
            configuration["Type"] = "individual"
            configuration.removeValue(forKey: "Linked")
            changed = true
        }
        for modeName in ["Desktop", "Idle"] {
            var mode = configuration[modeName] as? [String: Any] ?? [:]
            var content = mode["Content"] as? [String: Any] ?? [:]
            let choiceChanged = setAerialChoice(slotID, in: &content)
            if choiceChanged || mode["LastSet"] == nil || mode["LastUse"] == nil {
                // These dates are required by macOS' StoreIndex decoder, including new displays.
                mode["LastSet"] = Date()
                mode["LastUse"] = Date()
                mode["Content"] = content
                configuration[modeName] = mode
                changed = true
            }
        }
        return changed
    }

    private static func setLinkedAerialWallpaper(_ slotID: String, in configuration: inout [String: Any]) -> Bool {
        var changed = false
        if configuration["Type"] as? String != "linked" {
            configuration["Type"] = "linked"
            configuration.removeValue(forKey: "Desktop")
            configuration.removeValue(forKey: "Idle")
            changed = true
        }
        var linked = configuration["Linked"] as? [String: Any] ?? [:]
        var content = linked["Content"] as? [String: Any] ?? [:]
        let choiceChanged = setAerialChoice(slotID, in: &content)
        if choiceChanged || linked["LastSet"] == nil || linked["LastUse"] == nil {
            linked["LastSet"] = Date()
            linked["LastUse"] = Date()
            linked["Content"] = content
            configuration["Linked"] = linked
            changed = true
        } else if configuration["Linked"] == nil {
            linked["Content"] = content
            configuration["Linked"] = linked
            changed = true
        }
        return changed
    }

    private static func setAerialChoice(_ slotID: String, in content: inout [String: Any]) -> Bool {
        let missingOptions = content["Shuffle"] == nil || content["EncodedOptionValues"] == nil
        if content["Shuffle"] == nil { content["Shuffle"] = "$null" }
        if content["EncodedOptionValues"] == nil { content["EncodedOptionValues"] = "$null" }
        let currentChoices = content["Choices"] as? [[String: Any]] ?? []
        let alreadySet = currentChoices.count == 1
            && (currentChoices[0]["Provider"] as? String) == "com.apple.wallpaper.choice.aerials"
            && ((currentChoices[0]["Configuration"] as? Data).flatMap {
                (try? PropertyListSerialization.propertyList(from: $0, options: [], format: nil) as? [String: Any])?["assetID"] as? String
            } == slotID)
        if alreadySet { return missingOptions }
        guard let configurationData = try? PropertyListSerialization.data(
                fromPropertyList: ["assetID": slotID], format: .binary, options: 0
              ) else { return false }
        content["Choices"] = [[
            "Provider": "com.apple.wallpaper.choice.aerials",
            "Files": [],
            "Configuration": configurationData,
        ]]
        return true
    }

    private static func applyAerialSlots(
        _ slots: [String: String],
        defaultDisplayUUID: String?,
        to index: inout [String: Any]
    ) -> Bool {
        guard !slots.isEmpty else { return false }
        var changed = false
        let fallbackID = defaultDisplayUUID.flatMap({ slots[$0] }) ?? slots.keys.sorted().first.flatMap({ slots[$0] })

        var displays = index["Displays"] as? [String: Any] ?? [:]
        for displayUUID in slots.keys.sorted() {
            guard let slotID = slots[displayUUID] else { continue }
            var display = displays[displayUUID] as? [String: Any] ?? [:]
            if setAerialWallpaper(slotID, in: &display) { changed = true }
            displays[displayUUID] = display
        }
        index["Displays"] = displays

        if var spaces = index["Spaces"] as? [String: Any] {
            for spaceID in spaces.keys.sorted() {
                guard var space = spaces[spaceID] as? [String: Any] else { continue }
                if var displayMap = space["Displays"] as? [String: Any] {
                    for displayUUID in slots.keys.sorted() {
                        guard let slotID = slots[displayUUID] else { continue }
                        var display = displayMap[displayUUID] as? [String: Any] ?? [:]
                        if setAerialWallpaper(slotID, in: &display) { changed = true }
                        displayMap[displayUUID] = display
                    }
                    space["Displays"] = displayMap
                }
                if let fallbackID,
                   var defaultConfiguration = space["Default"] as? [String: Any],
                   setAerialWallpaper(fallbackID, in: &defaultConfiguration) {
                    space["Default"] = defaultConfiguration
                    changed = true
                }
                spaces[spaceID] = space
            }
            index["Spaces"] = spaces
        }

        if let fallbackID {
            // A global choice overrides every display and Space, even when Displays is populated.
            // This is the same nil encoding used when System Settings disables "all Spaces".
            if index["AllSpacesAndDisplays"] as? String != "$null" {
                index["AllSpacesAndDisplays"] = "$null"
                changed = true
            }
            var systemDefault = index["SystemDefault"] as? [String: Any] ?? [:]
            if setLinkedAerialWallpaper(fallbackID, in: &systemDefault) {
                index["SystemDefault"] = systemDefault
                changed = true
            }
        }
        return changed
    }

    private static func isProcessRunning(_ name: String) -> Bool {
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/pgrep")
        process.arguments = ["-x", name]
        process.standardOutput = FileHandle.nullDevice
        process.standardError = FileHandle.nullDevice
        guard (try? process.run()) != nil else { return false }
        process.waitUntilExit()
        return process.terminationStatus == 0
    }

    private static func restartWallpaperAgent() throws {
        // SIGTERM flushes the agent's old in-memory selection over our new Index.plist.
        // Relaunch its owner after the atomic write. Never kill the Aerial extension:
        // on macOS 27 that leaves the agent holding a dead XPC proxy (error 4099).
        let process = Process()
        process.executableURL = URL(fileURLWithPath: "/usr/bin/killall")
        process.arguments = ["-KILL", "WallpaperAgent"]
        process.standardOutput = FileHandle.nullDevice
        process.standardError = FileHandle.nullDevice
        try process.run()
        process.waitUntilExit()
        for _ in 0..<100 {
            usleep(100_000)
            if isProcessRunning("WallpaperAgent") && isProcessRunning("WallpaperAerialsExtension") { return }
        }
        throw NSError(domain: "AuraWallpaper", code: 4, userInfo: [NSLocalizedDescriptionKey: "macOS did not start its lock-screen video player"])
    }

    func setPause(_ pause: Bool) {
        isPaused = pause
        if isPaused {
            queuePlayer?.pause()
        } else {
            queuePlayer?.play()
        }
    }

    func togglePause() -> Bool {
        setPause(!isPaused)
        return isPaused
    }

    func setVolume(_ volPercent: Int) {
        volume = max(0.0, min(1.0, Float(volPercent) / 100.0))
        queuePlayer?.volume = volume
    }

    func setMute(_ mute: Bool) {
        isMuted = mute
        queuePlayer?.isMuted = mute
    }

    func setPanscan(_ panscan: Float) {
        if panscan >= 0.9 {
            scalingMode = "fill"
        } else {
            scalingMode = "fit"
        }
        applyScaling()
    }
}

class IpcServer {
    let socketPath: String
    let player: WallpaperPlayer
    var serverFd: Int32 = -1
    var isRunning = true

    init(socketPath: String, player: WallpaperPlayer) {
        self.socketPath = socketPath
        self.player = player
    }

    func start() {
        unlink(socketPath)

        let fd = socket(AF_UNIX, SOCK_STREAM, 0)
        guard fd >= 0 else {
            perror("[aura-bg] socket error")
            return
        }
        self.serverFd = fd

        var addr = sockaddr_un()
        addr.sun_family = sa_family_t(AF_UNIX)

        let pathBytes = socketPath.utf8CString
        guard pathBytes.count < MemoryLayout.size(ofValue: addr.sun_path) else {
            fputs("[aura-bg] Socket path too long\n", stderr)
            close(fd)
            return
        }

        withUnsafeMutablePointer(to: &addr.sun_path) { ptr in
            ptr.withMemoryRebound(to: CChar.self, capacity: pathBytes.count) { dest in
                _ = pathBytes.withUnsafeBufferPointer { src in
                    memcpy(dest, src.baseAddress!, src.count)
                }
            }
        }

        let addrLen = socklen_t(MemoryLayout<sockaddr_un>.size)
        let bindResult = withUnsafePointer(to: &addr) { ptr in
            ptr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sa in
                bind(fd, sa, addrLen)
            }
        }

        guard bindResult == 0 else {
            perror("[aura-bg] bind error")
            close(fd)
            return
        }

        guard listen(fd, 8) == 0 else {
            perror("[aura-bg] listen error")
            close(fd)
            return
        }

        DispatchQueue.global(qos: .userInitiated).async { [weak self] in
            self?.acceptLoop()
        }
    }

    func acceptLoop() {
        while isRunning && serverFd >= 0 {
            var clientAddr = sockaddr_un()
            var clientLen = socklen_t(MemoryLayout<sockaddr_un>.size)
            let clientFd = withUnsafeMutablePointer(to: &clientAddr) { ptr in
                ptr.withMemoryRebound(to: sockaddr.self, capacity: 1) { sa in
                    accept(serverFd, sa, &clientLen)
                }
            }

            guard clientFd >= 0 else {
                if !isRunning { break }
                usleep(50_000)
                continue
            }

            DispatchQueue.global(qos: .userInitiated).async { [weak self] in
                self?.handleClient(clientFd)
            }
        }
    }

    func handleClient(_ clientFd: Int32) {
        defer { close(clientFd) }

        var buffer = [UInt8](repeating: 0, count: 4096)
        while true {
            let bytesRead = read(clientFd, &buffer, buffer.count - 1)
            guard bytesRead > 0 else { break }
            buffer[bytesRead] = 0

            guard let rawStr = String(bytes: buffer[0..<bytesRead], encoding: .utf8) else { continue }
            for line in rawStr.components(separatedBy: "\n") {
                let trimmed = line.trimmingCharacters(in: .whitespacesAndNewlines)
                if trimmed.isEmpty { continue }

                if let resp = processJsonCommand(trimmed) {
                    let respData = (resp + "\n").data(using: .utf8)!
                    _ = respData.withUnsafeBytes { ptr in
                        write(clientFd, ptr.baseAddress!, respData.count)
                    }
                }
            }
        }
    }

    func processJsonCommand(_ jsonStr: String) -> String? {
        guard let data = jsonStr.data(using: .utf8),
              let json = try? JSONSerialization.jsonObject(with: data) as? [String: Any],
              let cmdArray = json["command"] as? [Any],
              let action = cmdArray.first as? String else {
            return "{\"error\":\"invalid command\"}"
        }

        switch action {
        case "loadfile":
            if cmdArray.count >= 2, let filePath = cmdArray[1] as? String {
                DispatchQueue.main.async { [weak self] in
                    self?.player.loadMedia(path: filePath)
                }
                return "{\"error\":\"success\"}"
            }
            return "{\"error\":\"missing path\"}"

        case "get_property":
            if cmdArray.count >= 2, let prop = cmdArray[1] as? String {
                switch prop {
                case "pause":
                    let paused = player.isPaused
                    return "{\"data\":\(paused),\"error\":\"success\"}"
                case "pid":
                    let pid = ProcessInfo.processInfo.processIdentifier
                    return "{\"data\":\(pid),\"error\":\"success\"}"
                case "volume":
                    let vol = Int(player.volume * 100.0)
                    return "{\"data\":\(vol),\"error\":\"success\"}"
                case "mute":
                    return "{\"data\":\(player.isMuted),\"error\":\"success\"}"
                default:
                    return "{\"data\":null,\"error\":\"success\"}"
                }
            }

        case "set_property":
            if cmdArray.count >= 3, let prop = cmdArray[1] as? String {
                let val = cmdArray[2]
                DispatchQueue.main.async { [weak self] in
                    guard let self = self else { return }
                    switch prop {
                    case "pause":
                        if let b = val as? Bool {
                            self.player.setPause(b)
                        }
                    case "mute":
                        if let b = val as? Bool {
                            self.player.setMute(b)
                        }
                    case "volume":
                        if let num = val as? NSNumber {
                            self.player.setVolume(num.intValue)
                        }
                    case "panscan":
                        if let num = val as? NSNumber {
                            self.player.setPanscan(num.floatValue)
                        }
                    case "keepaspect":
                        if let b = val as? Bool, !b {
                            self.player.scalingMode = "stretch"
                            self.player.applyScaling()
                        }
                    default:
                        break
                    }
                }
                return "{\"error\":\"success\"}"
            }

        case "cycle":
            if cmdArray.count >= 2, let prop = cmdArray[1] as? String {
                if prop == "pause" {
                    var newPaused = false
                    DispatchQueue.main.sync { [weak self] in
                        newPaused = self?.player.togglePause() ?? false
                    }
                    return "{\"data\":\(newPaused),\"error\":\"success\"}"
                }
            }

        case "quit":
            DispatchQueue.main.async {
                NSApp.terminate(nil)
            }
            return "{\"error\":\"success\"}"

        default:
            return "{\"error\":\"unknown command\"}"
        }

        return "{\"error\":\"success\"}"
    }

    func stop() {
        isRunning = false
        if serverFd >= 0 {
            close(serverFd)
            serverFd = -1
        }
        unlink(socketPath)
    }
}

// MARK: - Main Entry Point

let args = CommandLine.arguments
let app = NSApplication.shared
app.setActivationPolicy(.accessory)

if args.count >= 3, args[1] == "--prepare-lockscreen" {
    signal(SIGPIPE, SIG_IGN)
    let player = WallpaperPlayer()
    let screens = player.setupScreens(outputName: args.count > 3 ? args[3] : "*")
    player.targetScreens = screens
    let displayUUIDs = screens.compactMap { player.displayUUID(for: $0) }
    let defaultDisplayUUID = NSScreen.main.flatMap { player.displayUUID(for: $0) }
    let succeeded = player.syncLockScreenVideo(
        from: URL(fileURLWithPath: args[2]),
        displayUUIDs: displayUUIDs,
        defaultDisplayUUID: defaultDisplayUUID,
        progress: { value in
            let percent = Int((max(0, min(1, value)) * 100).rounded())
            let line = Data("LOCKSCREEN_PROGRESS:\(percent)\n".utf8)
            line.withUnsafeBytes { bytes in
                if let baseAddress = bytes.baseAddress {
                    _ = Darwin.write(STDOUT_FILENO, baseAddress, bytes.count)
                }
            }
        }
    )
    exit(succeeded ? 0 : 1)
}

// 1. Monitor detection request: -d or --list-monitors
if args.contains("-d") || args.contains("--list-monitors") || args.contains("--list-screens") {
    var seen = Set<String>()
    for (i, screen) in NSScreen.screens.enumerated() {
        var name = screen.localizedName
        if seen.contains(name) {
            name = "\(name) (\(i + 1))"
        }
        seen.insert(name)
        let width = Int(screen.frame.width)
        let height = Int(screen.frame.height)
        // Format compatible with mpvpaper -d output
        print("[*] Output: \(name)")
        print("    Resolution: \(width)x\(height)")
    }
    exit(0)
}

// 2. Parse arguments for playback
var socketPath: String? = nil
var outputTarget: String = "*"
var mediaPath: String? = nil
var autoPause: Bool = false
var scaling: String = "fit"
var isMuted: Bool = true
var initialVolume: Int = 100

var idx = 1
while idx < args.count {
    let arg = args[idx]
    if arg == "-o" && idx + 1 < args.count {
        let optStr = args[idx + 1]
        idx += 1
        for part in optStr.components(separatedBy: " ") {
            if part.starts(with: "--input-ipc-server=") {
                socketPath = String(part.dropFirst("--input-ipc-server=".count))
            } else if part.starts(with: "--volume=") {
                let vStr = String(part.dropFirst("--volume=".count))
                initialVolume = Int(vStr) ?? 100
            } else if part == "--mute=yes" || part == "--mute" {
                isMuted = true
            } else if part == "--mute=no" {
                isMuted = false
            } else if part.starts(with: "--panscan=1") {
                scaling = "fill"
            }
        }
    } else if arg == "-l" && idx + 1 < args.count {
        idx += 1 // skip layer argument
    } else if arg == "-p" {
        autoPause = true
    } else if !arg.starts(with: "-") {
        if mediaPath == nil && (arg.contains("/") || arg.contains(".")) && FileManager.default.fileExists(atPath: arg) {
            mediaPath = arg
        } else if outputTarget == "*" && !arg.contains("/") {
            outputTarget = arg
        } else if mediaPath == nil {
            mediaPath = arg
        }
    }
    idx += 1
}

guard let targetMedia = mediaPath, FileManager.default.fileExists(atPath: targetMedia) else {
    fputs("Usage: aura-bg [-d] [-o '<opts>'] [<output>] <media_path>\n", stderr)
    exit(1)
}

let player = WallpaperPlayer()
player.isMuted = isMuted
player.volume = max(0.0, min(1.0, Float(initialVolume) / 100.0))
player.scalingMode = scaling

let targetScreens = player.setupScreens(outputName: outputTarget)
player.targetScreens = targetScreens
for screen in targetScreens {
    let win = WallpaperWindow(screen: screen)
    player.windows.append(win)
}

player.loadMedia(path: targetMedia)

var ipcServer: IpcServer? = nil
if let sock = socketPath {
    let server = IpcServer(socketPath: sock, player: player)
    server.start()
    ipcServer = server
}

// Clean termination signals
signal(SIGTERM) { _ in
    NSApp.terminate(nil)
}
signal(SIGINT) { _ in
    NSApp.terminate(nil)
}

app.run()
