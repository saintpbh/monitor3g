# Monitor3G - Blackmagic Broadcast Application

Professional cross-platform broadcast application for Blackmagic UltraStudio Monitor 3G with support for multiple source types and SDI output.

## Features

### ✅ Current Features
- 🖥️  Professional Qt-based GUI
- 🎛️  Multiple output format support (1080p60, 720p60, etc.)
- 📋 Source management panel
- 🎥 Live preview window
- 🔧 Output control panel
- 📊 Comprehensive logging system
- ⚠️  Error handling and recovery

### 🚧 Planned Features
- 📹 Video file playback (MP4, MOV, AVI, etc.)
- 🖼️  Image display (PNG, JPG, etc.)
- 📄 Interactive PDF controller
- 🖥️  Screen capture (PPT, Keynote, Browser)
- 📷 Live camera input
- 🎨 Color space conversion (Rec.709, Rec.2020)

## Prerequisites

### 1. Required Software

#### macOS
- macOS 11.0 or later
- Xcode Command Line Tools
- Homebrew (recommended)

#### Windows
- Windows 10/11
- Visual Studio 2019 or later with C++ compiler
- vcpkg (recommended)

### 2. Dependencies

#### Qt 6
**macOS:**
```bash
brew install qt6
```

**Windows:**
Download and install from https://www.qt.io/download

#### FFmpeg
**macOS:**
```bash
brew install ffmpeg
```

**Windows:**
Use vcpkg or download pre-built binaries

#### Blackmagic DeckLink SDK

**Important: You must download the SDK manually from Blackmagic Design**

1. Visit: https://www.blackmagicdesign.com/support/
2. Search for "Desktop Video SDK"
3. Download the latest version (registration required)
4. Extract the SDK to `libs/decklink-sdk/` in this project

**Directory structure after extraction:**
```
Monitor3G/
└── libs/
    └── decklink-sdk/
        ├── Mac/
        │   └── include/
        │       ├── DeckLinkAPI.h
        │       └── ...
        └── Win/
            └── include/
                └── ...
```

## Building

### macOS

```bash
# Create build directory
mkdir build && cd build

# Configure with CMake
cmake .. -DCMAKE_PREFIX_PATH=$(brew --prefix qt6)

# Build
make -j$(sysctl -n hw.ncpu)

# Run
./bin/Monitor3G
```

### Windows

```bash
# Create build directory
mkdir build
cd build

# Configure with Visual Studio
cmake .. -G "Visual Studio 16 2019" -DCMAKE_PREFIX_PATH="C:/Qt/6.x.x/msvc2019_64"

# Build
cmake --build . --config Release

# Run
bin\Release\Monitor3G.exe
```

## Project Structure

```
Monitor3G/
├── CMakeLists.txt          # Root CMake configuration
├── README.md               # This file
├── cmake/                  # CMake modules
│   └── FindDeckLink.cmake  # DeckLink SDK finder
├── src/
│   ├── main.cpp           # Application entry point
│   ├── core/              # DeckLink SDK integration
│   │   ├── DeckLinkDevice.*
│   │   ├── DeckLinkOutput.*
│   │   └── DeviceMonitor.*
│   ├── sources/           # Source handlers
│   │   ├── AbstractSource.*
│   │   ├── VideoFileSource.*
│   │   ├── ImageSource.*
│   │   ├── PDFSource.*
│   │   └── ScreenCaptureSource.*
│   ├── output/            # Output management
│   │   ├── OutputFormat.*
│   │   └── OutputConfig.*
│   ├── ui/                # User interface
│   │   ├── MainWindow.*
│   │   ├── SourcePanel.*
│   │   ├── PreviewWidget.*
│   │   └── ControlPanel.*
│   └── utils/             # Utilities
│       ├── Logger.*
│       └── ErrorHandler.*
├── libs/                  # Third-party libraries
│   └── decklink-sdk/      # (Download separately)
└── resources/             # Application resources
    ├── icons/
    └── shaders/
```

## Usage

### 1. Connect Your Device
- Connect Blackmagic UltraStudio Monitor 3G to your computer
- Ensure DeckLink drivers are installed
- Power on the device

### 2. Launch Application
- The application will auto-detect connected devices
- Select your output format from the dropdown
- Choose a source type (Video, Image, PDF, Screen Capture, Camera)

### 3. Start Output
- Click "▶ Start Output" to begin broadcasting
- Monitor status in the control panel
- View live preview in the main window

## Troubleshooting

### "No DeckLink devices found"
1. Check physical connection
2. Verify DeckLink drivers are installed: https://www.blackmagicdesign.com/support/
3. Restart application after connecting device

### Build Errors
- **Qt not found**: Set `CMAKE_PREFIX_PATH` to your Qt installation
- **DeckLink SDK not found**: Ensure SDK is in `libs/decklink-sdk/`
- **FFmpeg not found**: Install FFmpeg development libraries

### Logging
Logs are saved to `monitor3g.log` in the application directory. Check this file for detailed error information.

## Development Roadmap

- [x] Phase 1: Project setup and UI framework
- [ ] Phase 2: DeckLink output implementation
- [ ] Phase 3: Video file playback
- [ ] Phase 4: Advanced sources (PDF, Screen Capture)
- [ ] Phase 5: Stability and optimization
- [ ] Phase 6: Cross-platform testing and deployment

## License

This project is provided as-is for professional broadcast use.

**Third-Party Licenses:**
- Qt: LGPL v3 (open source) / Commercial
- FFmpeg: LGPL v2.1+ / GPL v2+
- Blackmagic DeckLink SDK: Blackmagic Design EULA

## Credits

Developed for professional broadcast applications using Blackmagic UltraStudio Monitor 3G.

## Support

For issues and feature requests, please check:
1. Application logs (`monitor3g.log`)
2. Blackmagic Design support documentation
3. Qt documentation: https://doc.qt.io/

---

**Version:** 1.0.0  
**Last Updated:** 2026-01-31
