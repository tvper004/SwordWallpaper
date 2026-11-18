# macOS video wallpaper integration

Aura's native helper incorporates local Aerial registration from
[CaveWall](https://github.com/Vahsir7/cavewall) and the temporal HEVC encoding approach from
[AlexisBCD/macos-custom-video-wallpaper-fix](https://github.com/AlexisBCD/macos-custom-video-wallpaper-fix).
The AVAssetReader → VideoToolbox → AVAssetWriter path preserves the temporal
sample groups required by the native Aerial player during lock/unlock transitions.

MIT License

Copyright (c) 2026 AlexisBCD
Copyright (c) 2026 Rishav Bairagya

Permission is hereby granted, free of charge, to any person obtaining a copy
of this software and associated documentation files (the "Software"), to deal
in the Software without restriction, including without limitation the rights
to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
copies of the Software, and to permit persons to whom the Software is
furnished to do so, subject to the following conditions:

The above copyright notice and this permission notice shall be included in all
copies or substantial portions of the Software.

THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
SOFTWARE.

## Integration references

- [aerial-manager](https://github.com/kimmjen/aerial-manager): native Aerial
  asset selection and background conversion queue.
- [wallall](https://github.com/TresChar/wallall): global wallpaper precedence over
  per-display entries. Aura clears that override to allow independent monitors.
- [tahoe-live-lockscreen](https://github.com/AzhadArshad/tahoe-live-lockscreen):
  video duration and extension lifecycle research. Its extension-kill workaround
  is deliberately not used: on the tested macOS 27.0.1 system it invalidated
  WallpaperAgent's XPC connection and left the lock screen empty.

Aura uses the native macOS Aerial renderer for the locked session and its own
AVPlayer for continuous desktop playback. macOS controls the lock-screen motion,
including stopping animation while the password field is active. This does not
replace the FileVault pre-boot authentication screen.
