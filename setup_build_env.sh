#!/bin/bash
# Monitor3G - 빌드 환경 자동 설치 스크립트

set -e

echo "🔧 Monitor3G 빌드 환경 설정을 시작합니다..."
echo ""

# Homebrew 확인
if ! command -v brew &> /dev/null; then
    echo "❌ Homebrew가 설치되지 않았습니다."
    echo "다음 명령으로 설치하세요:"
    echo '/bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"'
    exit 1
fi

echo "✅ Homebrew 설치 확인됨"

# CMake 설치
if ! command -v cmake &> /dev/null; then
    echo "📦 CMake 설치 중..."
    brew install cmake
else
    echo "✅ CMake 이미 설치됨 ($(cmake --version | head -1))"
fi

# Qt 6 확인
if [ -d "/opt/homebrew/opt/qt" ] || [ -d "/usr/local/opt/qt" ]; then
    echo "✅ Qt 6 이미 설치됨"
    QT_PATH=$(brew --prefix qt)
    echo "   경로: $QT_PATH"
else
    echo "📦 Qt 6 설치 중... (시간이 걸릴 수 있습니다)"
    brew install qt
fi

# pkg-config 설치
if ! command -v pkg-config &> /dev/null; then
    echo "📦 pkg-config 설치 중..."
    brew install pkg-config
else
    echo "✅ pkg-config 이미 설치됨"
fi

# FFmpeg 설치
if ! command -v ffmpeg &> /dev/null; then
    echo "📦 FFmpeg 설치 중..."
    brew install ffmpeg
else
    echo "✅ FFmpeg 이미 설치됨 ($(ffmpeg -version | head -1 | cut -d' ' -f3))"
fi

echo ""
echo "✅ 모든 빌드 도구 설치 완료!"
echo ""
echo "📋 설치된 도구:"
echo "   - CMake: $(cmake --version | head -1)"
echo "   - Qt: $(brew --prefix qt)"
echo "   - FFmpeg: $(ffmpeg -version | head -1 | cut -d' ' -f3)"
echo ""
echo "🚀 이제 다음 명령으로 프로젝트를 빌드할 수 있습니다:"
echo ""
echo "   mkdir build && cd build"
echo "   cmake .. -DCMAKE_PREFIX_PATH=\$(brew --prefix qt)"
echo "   make -j\$(sysctl -n hw.ncpu)"
echo "   ./bin/Monitor3G"
echo ""
