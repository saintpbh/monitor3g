# Blackmagic DeckLink SDK 다운로드 및 설치 가이드

이 프로젝트는 Blackmagic DeckLink SDK가 필요합니다. SDK는 무료이지만 Blackmagic Design 웹사이트에서 직접 다운로드해야 합니다.

## 단계별 설치 가이드

### 1단계: SDK 다운로드

1. **Blackmagic Support 페이지 방문**
   - 브라우저에서 https://www.blackmagicdesign.com/support/ 열기

2. **Desktop Video SDK 검색**
   - 검색창에 "Desktop Video SDK" 입력
   - 또는 Products → Capture & Playback → Desktop Video 메뉴 이동

3. **최신 SDK 다운로드**
   - "Latest Downloads" 섹션에서 "Desktop Video SDK" 찾기
   - Mac 또는 Windows 버전 선택
   - 다운로드 전 간단한 등록 필요 (무료)

### 2단계: SDK 압축 해제

다운로드한 .zip 또는 .dmg 파일을 압축 해제합니다.

macOS 예시:
```bash
# Downloads 폴더에서 압축 해제 (예시)
cd ~/Downloads
unzip Blackmagic_DeckLink_SDK_*.zip
```

Windows 예시:
- .zip 파일을 우클릭하여 "압축 풀기" 선택

### 3단계: SDK를 프로젝트에 복사

SDK의 include 파일들을 프로젝트의 `libs/decklink-sdk/` 폴더로 복사합니다.

**macOS:**
```bash
cd /Users/bongpark/Library/CloudStorage/OneDrive-한국기독교장로회총회유지재단/0.박봉환개인문서폴더/앱개발/Monitor3G

# SDK 폴더 생성
mkdir -p libs/decklink-sdk/Mac/include

# SDK 헤더 파일 복사 (압축 해제한 SDK 경로에 맞게 수정)
cp -r ~/Downloads/Blackmagic\ DeckLink\ SDK\ */Mac/include/* libs/decklink-sdk/Mac/include/
```

**Windows (PowerShell):**
```powershell
cd "C:\Users\...\Monitor3G"

# SDK 폴더 생성
New-Item -ItemType Directory -Force -Path libs\decklink-sdk\Win\include

# SDK 헤더 파일 복사 (압축 해제한 SDK 경로에 맞게 수정)
Copy-Item -Recurse "C:\Users\...\Downloads\Blackmagic DeckLink SDK\Win\include\*" "libs\decklink-sdk\Win\include\"
```

### 4단계: 설치 확인

올바르게 설치되었는지 확인:

```bash
# macOS
ls -la libs/decklink-sdk/Mac/include/

# 다음 파일들이 보여야 합니다:
# - DeckLinkAPI.h
# - DeckLinkAPIConfiguration.h
# - DeckLinkAPIDiscovery.h
# - DeckLinkAPIModes.h
# - DeckLinkAPITypes.h
```

```powershell
# Windows
dir libs\decklink-sdk\Win\include\

# 다음 파일들이 보여야 합니다:
# - DeckLinkAPI.h
# - DeckLinkAPI_i.c
# - DeckLinkAPI.idl
# 등등
```

### 5단계: DeckLink 드라이버 설치

SDK와 별도로 **DeckLink Desktop Video 드라이버**도 설치해야 합니다:

1. 같은 support 페이지에서 "Desktop Video" 드라이버 다운로드
2. macOS: .dmg 파일 실행하여 설치
3. Windows: .exe 파일 실행하여 설치
4. **컴퓨터 재시작** (필수!)

### 6단계: 하드웨어 연결 (선택사항)

Blackmagic UltraStudio Monitor 3G가 있다면:
1. 장치를 컴퓨터에 연결 (Thunderbolt/USB-C)
2. 전원 켜기
3. 시스템 환경설정/장치 관리자에서 장치 인식 확인

## 디렉토리 구조 예시

설치 완료 후 프로젝트 구조:

```
Monitor3G/
├── CMakeLists.txt
├── README.md
├── SDK_INSTALL.md          # 이 파일
├── libs/
│   └── decklink-sdk/
│       ├── Mac/
│       │   └── include/
│       │       ├── DeckLinkAPI.h
│       │       ├── DeckLinkAPIConfiguration.h
│       │       ├── DeckLinkAPIDiscovery.h
│       │       └── ... (기타 헤더 파일들)
│       └── Win/
│           └── include/
│               └── ... (Windows 헤더 파일들)
└── src/
    └── ...
```

## 문제 해결

### "SDK를 다운로드할 수 없습니다"
- Blackmagic Design 계정 등록이 필요합니다 (무료)
- 등록 후 다시 다운로드 시도

### "파일을 찾을 수 없습니다"
- ZIP 파일 압축이 완전히 해제되었는지 확인
- 경로에 특수문자나 공백이 있는지 확인

### "드라이버가 설치되지 않습니다"
- 관리자 권한으로 설치 프로그램 실행
- 보안 설정 확인 (macOS System Preferences → Security)
- 설치 후 반드시 재시작

## 다음 단계

SDK 설치가 완료되면:

1. `README.md`의 빌드 지침 확인
2. CMake 프로젝트 빌드
3. 애플리케이션 실행 및 테스트

## 라이센스 참고사항

Blackmagic DeckLink SDK는 Blackmagic Design의 EULA를 따릅니다.
- 개발 및 배포는 EULA 조건에 따라 허용됩니다
- 상업적 사용 시 라이센스 조건 확인 필요

---

도움이 필요하면 Blackmagic Design 공식 문서를 참조하세요:
https://documents.blackmagicdesign.com/
