cmake_minimum_required(VERSION 3.20)
project(Monitor3G VERSION 1.0.6 LANGUAGES CXX)

# C++ Standard
set(CMAKE_CXX_STANDARD 17)
set(CMAKE_CXX_STANDARD_REQUIRED ON)

# Qt Configuration
set(CMAKE_AUTOMOC ON)
set(CMAKE_AUTORCC ON)
set(CMAKE_AUTOUIC ON)

# Add cmake module path
list(APPEND CMAKE_MODULE_PATH "${CMAKE_SOURCE_DIR}/cmake")

# Find Qt6
find_package(Qt6 REQUIRED COMPONENTS Core Widgets Gui Multimedia OpenGLWidgets)

# Find FFmpeg
find_package(PkgConfig REQUIRED)
pkg_check_modules(FFMPEG REQUIRED 
    libavcodec 
    libavformat 
    libavutil 
    libswscale
    libswresample
)

# Find DeckLink SDK
find_package(DeckLink REQUIRED)
find_package(DeckLink REQUIRED)
message(STATUS "DECKLINK_INCLUDE_DIRS: ${DECKLINK_INCLUDE_DIRS}")
message(STATUS "DECKLINK_LIBRARIES: ${DECKLINK_LIBRARIES}")

# Platform-specific settings
if(APPLE)
    set(CMAKE_OSX_DEPLOYMENT_TARGET "11.0")
    find_library(COREFOUNDATION_LIBRARY CoreFoundation)
    find_library(COREVIDEO_LIBRARY CoreVideo)
    find_library(COREMEDIA_LIBRARY CoreMedia)
    find_library(SCREENCAPTUREKIT_LIBRARY ScreenCaptureKit)
    set(PLATFORM_LIBS 
        ${COREFOUNDATION_LIBRARY}
        ${COREVIDEO_LIBRARY}
        ${COREMEDIA_LIBRARY}
        ${SCREENCAPTUREKIT_LIBRARY}
    )
elseif(WIN32)
    set(PLATFORM_LIBS ole32 oleaut32)
endif()

# Include directories
include_directories(
    ${CMAKE_SOURCE_DIR}/src
    ${FFMPEG_INCLUDE_DIRS}
    ${DECKLINK_INCLUDE_DIRS}
    ${CMAKE_SOURCE_DIR}/libs/decklink-sdk/Mac/include
    ${Qt6_OPENGL_INCLUDE_DIRS}
)

# Source files
set(SOURCES
    src/main.cpp
    src/core/DeckLinkDevice.cpp
    src/core/DeckLinkOutput.cpp
    src/core/DeviceMonitor.cpp
    src/sources/AbstractSource.cpp
    src/sources/VideoFileSource.cpp
    src/sources/ImageSource.cpp
    src/sources/LiveCameraSource.cpp
    src/sources/ScreenCaptureSource.cpp
    src/sources/PDFSource.cpp
    src/sources/SourceManager.cpp
    src/sources/TestPatternSource.cpp
    src/output/OutputFormat.cpp
    src/output/OutputConfig.cpp
    src/ui/MainWindow.cpp
    src/ui/SourcePanel.cpp
    src/ui/PreviewWidget.cpp
    src/ui/ControlPanel.cpp
    src/ui/PDFNavigator.cpp
    src/utils/Logger.cpp
    src/utils/ErrorHandler.cpp
    src/ui/DeveloperConsole.cpp
    libs/decklink-sdk/Mac/include/DeckLinkAPIDispatch.cpp
)

# Header files
set(HEADERS
    src/core/DeckLinkDevice.h
    src/core/DeckLinkOutput.h
    src/core/DeviceMonitor.h
    src/sources/AbstractSource.h
    src/sources/VideoFileSource.h
    src/sources/ImageSource.h
    src/sources/LiveCameraSource.h
    src/sources/ScreenCaptureSource.h
    src/sources/PDFSource.h
    src/sources/SourceManager.h
    src/sources/TestPatternSource.h
    src/output/OutputFormat.h
    src/output/OutputConfig.h
    src/ui/MainWindow.h
    src/ui/SourcePanel.h
    src/ui/PreviewWidget.h
    src/ui/ControlPanel.h
    src/ui/PDFNavigator.h
    src/ui/DeveloperConsole.h
    src/utils/Logger.h
    src/utils/LogMessenger.h
    src/utils/ErrorHandler.h
)

# Create executable
add_executable(${PROJECT_NAME} ${SOURCES} ${HEADERS})

# Link libraries
target_link_libraries(${PROJECT_NAME} PRIVATE
    Qt6::Core
    Qt6::Widgets
    Qt6::Gui
    Qt6::Multimedia
    Qt6::OpenGLWidgets
    ${FFMPEG_LIBRARY_DIRS}/libavformat.dylib
    ${FFMPEG_LIBRARY_DIRS}/libavcodec.dylib
    ${FFMPEG_LIBRARY_DIRS}/libswscale.dylib
    ${FFMPEG_LIBRARY_DIRS}/libswresample.dylib
    ${FFMPEG_LIBRARY_DIRS}/libavutil.dylib
    ${DECKLINK_LIBRARIES}
    ${PLATFORM_LIBS}
)

# Set output directory
set_target_properties(${PROJECT_NAME} PROPERTIES
    RUNTIME_OUTPUT_DIRECTORY "${CMAKE_BINARY_DIR}/bin"
)

# Installation
install(TARGETS ${PROJECT_NAME} DESTINATION bin)

# macOS Bundle
if(APPLE)
    set_target_properties(${PROJECT_NAME} PROPERTIES
        MACOSX_BUNDLE TRUE
        MACOSX_BUNDLE_GUI_IDENTIFIER "com.monitor3g.app"
        MACOSX_BUNDLE_BUNDLE_NAME "Monitor3G"
        MACOSX_BUNDLE_BUNDLE_VERSION ${PROJECT_VERSION}
        MACOSX_BUNDLE_SHORT_VERSION_STRING ${PROJECT_VERSION}
    )
endif()
