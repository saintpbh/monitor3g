#include "TestPatternSource.h"
#include "../utils/Logger.h"
#include <QPainter>
#include <cstring>

namespace Monitor3G {

// SMPTE Color Bar YUV values
struct ColorYUV {
  uint8_t y, u, v;
};

static const ColorYUV SMPTE_COLORS[] = {
    {235, 128, 128}, // White (100%)
    {210, 16, 146},  // Yellow
    {170, 166, 16},  // Cyan
    {145, 54, 34},   // Green
    {106, 202, 222}, // Magenta
    {81, 90, 240},   // Red
    {41, 240, 110},  // Blue
    {16, 128, 128}   // Black
};

TestPatternSource::TestPatternSource(QObject *parent)
    : AbstractSource(parent), m_pattern(TestPattern::ColorBars),
      m_isActive(false), m_frameWidth(1920), m_frameHeight(1080) {}

TestPatternSource::~TestPatternSource() { stop(); }

bool TestPatternSource::start() {
  if (m_isActive)
    return true;
  LOG_INFO("Starting test pattern source");
  m_isActive = true;
  emit started();
  return true;
}

void TestPatternSource::stop() {
  if (!m_isActive)
    return;
  LOG_INFO("Stopping test pattern source");
  m_isActive = false;
  emit stopped();
}

bool TestPatternSource::fillNextFrame(IDeckLinkMutableVideoFrame *frame) {
  LOG_INFO(
      QString(
          "TestPatternSource::fillNextFrame called, m_isActive=%1, frame=%2")
          .arg(m_isActive)
          .arg(frame ? "valid" : "null"));

  if (!m_isActive || !frame)
    return false;

  long width = frame->GetWidth();
  long height = frame->GetHeight();

  LOG_INFO(QString("Frame dimensions: %1x%2").arg(width).arg(height));

  IDeckLinkVideoBuffer *videoBuffer = nullptr;
  if (frame->QueryInterface(IID_IDeckLinkVideoBuffer, (void **)&videoBuffer) ==
      S_OK) {
    void *buffer = nullptr;

    // On macOS/Desktop Video SDK, we must call StartAccess before GetBytes
    if (videoBuffer->StartAccess(bmdBufferAccessWrite) == S_OK) {
      if (videoBuffer->GetBytes(&buffer) == S_OK && buffer) {
        LOG_INFO(
            QString("Generating pattern %1").arg(static_cast<int>(m_pattern)));
        switch (m_pattern) {
        case TestPattern::ColorBars:
          generateColorBars(buffer, width, height);
          break;
        case TestPattern::BlackFrame:
          generateBlackFrame(buffer, width, height);
          break;
        case TestPattern::WhiteFrame:
          generateWhiteFrame(buffer, width, height);
          break;
        case TestPattern::Checkerboard:
          generateCheckerboard(buffer, width, height);
          break;
        }
        LOG_INFO("Pattern generation complete");

        videoBuffer->EndAccess(bmdBufferAccessWrite);
        videoBuffer->Release();
        return true;
      }
      videoBuffer->EndAccess(bmdBufferAccessWrite);
    } else {
      LOG_ERROR("Failed to StartAccess on video buffer");
    }
    videoBuffer->Release();
  } else {
    // If QueryInterface fails, let's log the error explicitly
    LOG_ERROR("Failed to get IDeckLinkVideoBuffer interface from frame via "
              "QueryInterface");
  }

  return false;
}

void TestPatternSource::generateColorBars(void *buffer, int width, int height) {
  // SMPTE RP 219-1:2014 Color Bar Colors (at 75% amplitude)
  // 8-bit Y, Cb, Cr values (Legal Range: Y 16-235, C 16-240)
  static const struct {
    uint8_t y, u, v;
  } s_colors[] = {
      {180, 128, 128}, // White (75%)
      {162, 44, 142},  // Yellow
      {131, 156, 44},  // Cyan
      {112, 72, 58},   // Green
      {84, 184, 198},  // Magenta
      {65, 100, 212},  // Red
      {35, 212, 114},  // Blue
      {16, 128, 128}   // Black
  };

  uint8_t *pixels = static_cast<uint8_t *>(buffer);
  long rowBytes = width * 2; // UYVY: 2 bytes per pixel

  // Top section (3/4 of height): Standard color bars
  int topHeight = height * 3 / 4;
  for (int y = 0; y < topHeight; ++y) {
    uint8_t *row = pixels + y * rowBytes;
    for (int x = 0; x < width; x += 2) {
      // Simple vertical bars
      int barIndex = (x * 8) / width;
      if (barIndex > 7)
        barIndex = 7;

      auto color = s_colors[barIndex];

      // UYVY: [U0] [Y0] [V0] [Y1] for every 2 pixels (4 bytes)
      int pixelOffset = x * 2;
      row[pixelOffset + 0] = color.u; // U
      row[pixelOffset + 1] = color.y; // Y0
      row[pixelOffset + 2] = color.v; // V
      row[pixelOffset + 3] = color.y; // Y1
    }
  }

  // Middle section: Black, White, Black, Gray (1/8 of height)
  int midStart = topHeight;
  int midHeight = height / 8;

  for (int y = midStart; y < midStart + midHeight; y++) {
    uint8_t *row = pixels + y * rowBytes;
    for (int x = 0; x < width; x += 2) {
      ColorYUV color;

      if (x < width / 4) {
        color = {16, 128, 128}; // Black
      } else if (x < width / 2) {
        color = {235, 128, 128}; // White
      } else if (x < width * 3 / 4) {
        color = {16, 128, 128}; // Black
      } else {
        color = {128, 128, 128}; // 50% Gray
      }

      // UYVY: [U0] [Y0] [V0] [Y1]
      int pixelOffset = x * 2;
      row[pixelOffset + 0] = color.u; // U
      row[pixelOffset + 1] = color.y; // Y0
      row[pixelOffset + 2] = color.v; // V
      row[pixelOffset + 3] = color.y; // Y1
    }
  }

  // Bottom section: PLUGE pattern (remaining)
  int bottomStart = midStart + midHeight;

  for (int y = bottomStart; y < height; y++) {
    uint8_t *row = pixels + y * rowBytes;
    for (int x = 0; x < width; x += 2) {
      ColorYUV color;

      int section = (x * 7) / width;

      switch (section) {
      case 0:
        color = {16, 128, 128}; // Black (was Super-black)
        break;
      case 1:
        color = {16, 128, 128}; // Black
        break;
      case 2:
        color = {128, 128, 128}; // 50% Gray
        break;
      case 3:
        color = {16, 128, 128}; // Black
        break;
      case 4:
        color = {4, 128, 128}; // -4dB
        break;
      case 5:
        color = {16, 128, 128}; // Black
        break;
      default:
        color = {20, 128, 128}; // +4dB
        break;
      }

      // UYVY: [U0] [Y0] [V0] [Y1]
      int pixelOffset = x * 2;
      row[pixelOffset + 0] = color.u; // U
      row[pixelOffset + 1] = color.y; // Y0
      row[pixelOffset + 2] = color.v; // V
      row[pixelOffset + 3] = color.y; // Y1
    }
  }
}

void TestPatternSource::generateBlackFrame(void *buffer, int width,
                                           int height) {
  uint8_t *pixels = static_cast<uint8_t *>(buffer);

  // UYVY: Each 4 bytes represents 2 pixels: [U0] [Y0] [V0] [Y1]
  // Black: Y=16, U=128, V=128
  for (int i = 0; i < width * height * 2; i += 4) {
    pixels[i + 0] = 128; // U
    pixels[i + 1] = 16;  // Y0 (black)
    pixels[i + 2] = 128; // V
    pixels[i + 3] = 16;  // Y1 (black)
  }
}

void TestPatternSource::generateWhiteFrame(void *buffer, int width,
                                           int height) {
  LOG_INFO("Generating white frame");
  uint8_t *pixels = static_cast<uint8_t *>(buffer);

  // UYVY: Each 4 bytes represents 2 pixels: [U0] [Y0] [V0] [Y1]
  // White: Y=235, U=128, V=128
  for (int i = 0; i < width * height * 2; i += 4) {
    pixels[i + 0] = 128; // U
    pixels[i + 1] = 235; // Y0 (white)
    pixels[i + 2] = 128; // V
    pixels[i + 3] = 235; // Y1 (white)
  }
  LOG_INFO("White frame generation complete");
}

void TestPatternSource::generateCheckerboard(void *buffer, int width,
                                             int height) {
  uint8_t *pixels = static_cast<uint8_t *>(buffer);
  int squareSize = 64;
  long rowBytes = width * 2;

  for (int y = 0; y < height; y++) {
    uint8_t *row = pixels + y * rowBytes;
    for (int x = 0; x < width; x += 2) {
      // Both pixels in pair share same square color for simplicity
      bool isWhite = ((x / squareSize) + (y / squareSize)) % 2 == 0;
      uint8_t yValue = isWhite ? 235 : 16;

      // UYVY: [U0] [Y0] [V0] [Y1]
      int pixelOffset = x * 2;
      row[pixelOffset + 0] = 128;    // U
      row[pixelOffset + 1] = yValue; // Y0
      row[pixelOffset + 2] = 128;    // V
      row[pixelOffset + 3] = yValue; // Y1
    }
  }
}

void TestPatternSource::setYUV(uint8_t *buffer, uint8_t y, uint8_t u,
                               uint8_t v) {
  buffer[0] = y;
  buffer[1] = u;
  buffer[2] = y;
  buffer[3] = v;
}

void TestPatternSource::fillQImage(QImage &image) {
  int w = image.width();
  int h = image.height();

  QPainter p(&image);
  p.fillRect(0, 0, w, h, Qt::black);

  // Draw bars
  p.fillRect(0, 0, w / 7, h, Qt::gray);
  p.fillRect(w / 7, 0, w / 7, h, Qt::yellow);
  p.fillRect(2 * w / 7, 0, w / 7, h, Qt::cyan);
  p.fillRect(3 * w / 7, 0, w / 7, h, Qt::green);
  p.fillRect(4 * w / 7, 0, w / 7, h, Qt::magenta);
  p.fillRect(5 * w / 7, 0, w / 7, h, Qt::red);
  p.fillRect(6 * w / 7, 0, w / 7, h, Qt::blue);

  // Moving indicator
  static int pos = 0;
  if (w > 0) {
    pos = (pos + 5) % w;
    p.fillRect(pos, h / 2 - 10, 20, 20, Qt::white);
  }

  p.end();
}

} // namespace Monitor3G
