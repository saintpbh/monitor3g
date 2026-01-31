#include "MultiViewSource.h"
#include "../utils/Logger.h"
#include "DeckLinkAPI.h"
#include <QPainter>

namespace Monitor3G {

MultiViewSource::MultiViewSource(QObject *parent)
    : AbstractSource(parent), m_name("Multi-View"), m_isActive(false),
      m_layout(MultiViewLayout::PictureInPicture), m_pipX(1440), m_pipY(30),
      m_pipWidth(450), m_pipHeight(253) {
  // Initialize with black canvas
  m_compositedImage =
      QImage(TARGET_WIDTH, TARGET_HEIGHT, QImage::Format_ARGB32);
  m_compositedImage.fill(Qt::black);

  // Pre-allocate UYVY buffer
  m_uyvyData.resize(TARGET_WIDTH * TARGET_HEIGHT * 2);
}

MultiViewSource::~MultiViewSource() { stop(); }

bool MultiViewSource::start() {
  QMutexLocker locker(&m_mutex);
  m_isActive = true;
  LOG_INFO("MultiViewSource started");
  emit started();
  return true;
}

void MultiViewSource::stop() {
  QMutexLocker locker(&m_mutex);
  m_isActive = false;
  LOG_INFO("MultiViewSource stopped");
  emit stopped();
}

void MultiViewSource::setLayout(MultiViewLayout layout) {
  QMutexLocker locker(&m_mutex);
  m_layout = layout;
  LOG_INFO(
      QString("MultiView layout changed to: %1").arg(static_cast<int>(layout)));
}

void MultiViewSource::setMainSource(std::shared_ptr<AbstractSource> source) {
  QMutexLocker locker(&m_mutex);
  m_sources[0] = source;
}

void MultiViewSource::setOverlaySource(std::shared_ptr<AbstractSource> source,
                                       int index) {
  QMutexLocker locker(&m_mutex);
  if (index >= 0 && index < 3) {
    m_sources[index + 1] = source;
  }
}

void MultiViewSource::setSource(int slot,
                                std::shared_ptr<AbstractSource> source) {
  QMutexLocker locker(&m_mutex);
  if (slot >= 0 && slot < 4) {
    m_sources[slot] = source;
  }
}

void MultiViewSource::clearSources() {
  QMutexLocker locker(&m_mutex);
  for (int i = 0; i < 4; ++i) {
    m_sources[i].reset();
  }
}

void MultiViewSource::setPipPosition(int x, int y) {
  QMutexLocker locker(&m_mutex);
  m_pipX = x;
  m_pipY = y;
}

void MultiViewSource::setPipSize(int width, int height) {
  QMutexLocker locker(&m_mutex);
  m_pipWidth = width;
  m_pipHeight = height;
}

void MultiViewSource::drawSourceToCanvas(const QImage &source, QImage &canvas,
                                         int x, int y, int w, int h) {
  if (source.isNull()) {
    // Draw placeholder (dark gray)
    QPainter painter(&canvas);
    painter.fillRect(x, y, w, h, QColor(30, 30, 30));
    painter.setPen(QColor(80, 80, 80));
    painter.drawRect(x, y, w - 1, h - 1);
    return;
  }

  // Scale source to fit target rect while maintaining aspect ratio
  QImage scaled =
      source.scaled(w, h, Qt::KeepAspectRatio, Qt::SmoothTransformation);

  // Center in target rect
  int offsetX = x + (w - scaled.width()) / 2;
  int offsetY = y + (h - scaled.height()) / 2;

  QPainter painter(&canvas);
  painter.fillRect(x, y, w, h, Qt::black);
  painter.drawImage(offsetX, offsetY, scaled);
}

void MultiViewSource::compositeFrame() {
  QMutexLocker locker(&m_mutex);

  // Clear canvas
  m_compositedImage.fill(Qt::black);

  // Get source frames
  QImage sourceImages[4];
  for (int i = 0; i < 4; ++i) {
    if (m_sources[i]) {
      sourceImages[i] =
          QImage(TARGET_WIDTH, TARGET_HEIGHT, QImage::Format_ARGB32);
      sourceImages[i].fill(Qt::black);
      m_sources[i]->fillQImage(sourceImages[i]);
    }
  }

  switch (m_layout) {
  case MultiViewLayout::PictureInPicture:
    // Main source takes full screen
    if (m_sources[0]) {
      drawSourceToCanvas(sourceImages[0], m_compositedImage, 0, 0, TARGET_WIDTH,
                         TARGET_HEIGHT);
    }
    // Overlay source in corner (PiP)
    if (m_sources[1]) {
      drawSourceToCanvas(sourceImages[1], m_compositedImage, m_pipX, m_pipY,
                         m_pipWidth, m_pipHeight);

      // Draw border around PiP
      QPainter painter(&m_compositedImage);
      painter.setPen(QPen(QColor(255, 255, 255), 2));
      painter.drawRect(m_pipX, m_pipY, m_pipWidth, m_pipHeight);
    }
    break;

  case MultiViewLayout::SideBySide: {
    int halfWidth = TARGET_WIDTH / 2;
    drawSourceToCanvas(sourceImages[0], m_compositedImage, 0, 0, halfWidth,
                       TARGET_HEIGHT);
    drawSourceToCanvas(sourceImages[1], m_compositedImage, halfWidth, 0,
                       halfWidth, TARGET_HEIGHT);
  } break;

  case MultiViewLayout::StackedVertical: {
    int halfHeight = TARGET_HEIGHT / 2;
    drawSourceToCanvas(sourceImages[0], m_compositedImage, 0, 0, TARGET_WIDTH,
                       halfHeight);
    drawSourceToCanvas(sourceImages[1], m_compositedImage, 0, halfHeight,
                       TARGET_WIDTH, halfHeight);
  } break;

  case MultiViewLayout::QuadView: {
    int halfWidth = TARGET_WIDTH / 2;
    int halfHeight = TARGET_HEIGHT / 2;

    drawSourceToCanvas(sourceImages[0], m_compositedImage, 0, 0, halfWidth,
                       halfHeight);
    drawSourceToCanvas(sourceImages[1], m_compositedImage, halfWidth, 0,
                       halfWidth, halfHeight);
    drawSourceToCanvas(sourceImages[2], m_compositedImage, 0, halfHeight,
                       halfWidth, halfHeight);
    drawSourceToCanvas(sourceImages[3], m_compositedImage, halfWidth,
                       halfHeight, halfWidth, halfHeight);

    // Draw grid lines
    QPainter painter(&m_compositedImage);
    painter.setPen(QPen(QColor(60, 60, 60), 2));
    painter.drawLine(halfWidth, 0, halfWidth, TARGET_HEIGHT);
    painter.drawLine(0, halfHeight, TARGET_WIDTH, halfHeight);
  } break;
  }
}

void MultiViewSource::convertToUYVY() {
  const uchar *rgb = m_compositedImage.constBits();
  int bytesPerLine = m_compositedImage.bytesPerLine();

  for (int y = 0; y < TARGET_HEIGHT; ++y) {
    const uchar *line = rgb + y * bytesPerLine;
    uint8_t *dest = m_uyvyData.data() + y * TARGET_WIDTH * 2;

    for (int x = 0; x < TARGET_WIDTH; x += 2) {
      // Get two RGB pixels
      int b1 = line[x * 4 + 0];
      int g1 = line[x * 4 + 1];
      int r1 = line[x * 4 + 2];

      int b2 = line[(x + 1) * 4 + 0];
      int g2 = line[(x + 1) * 4 + 1];
      int r2 = line[(x + 1) * 4 + 2];

      // RGB to YUV conversion
      int y1 = ((66 * r1 + 129 * g1 + 25 * b1 + 128) >> 8) + 16;
      int y2 = ((66 * r2 + 129 * g2 + 25 * b2 + 128) >> 8) + 16;
      int u = ((-38 * r1 - 74 * g1 + 112 * b1 + 128) >> 8) + 128;
      int v = ((112 * r1 - 94 * g1 - 18 * b1 + 128) >> 8) + 128;

      // Clamp values
      y1 = std::clamp(y1, 16, 235);
      y2 = std::clamp(y2, 16, 235);
      u = std::clamp(u, 16, 240);
      v = std::clamp(v, 16, 240);

      // UYVY format: U Y1 V Y2
      dest[x * 2 + 0] = static_cast<uint8_t>(u);
      dest[x * 2 + 1] = static_cast<uint8_t>(y1);
      dest[x * 2 + 2] = static_cast<uint8_t>(v);
      dest[x * 2 + 3] = static_cast<uint8_t>(y2);
    }
  }
}

bool MultiViewSource::fillNextFrame(IDeckLinkMutableVideoFrame *frame) {
  if (!frame || !m_isActive)
    return false;

  compositeFrame();
  convertToUYVY();

  // Query IDeckLinkVideoBuffer interface for GetBytes
  IDeckLinkVideoBuffer *videoBuffer = nullptr;
  if (frame->QueryInterface(IID_IDeckLinkVideoBuffer, (void **)&videoBuffer) ==
      S_OK) {
    void *buffer = nullptr;
    if (videoBuffer->StartAccess(bmdBufferAccessWrite) == S_OK) {
      if (videoBuffer->GetBytes(&buffer) == S_OK && buffer) {
        long rowBytes = frame->GetRowBytes();
        long height = frame->GetHeight();
        long expectedSize = rowBytes * height;

        if (m_uyvyData.size() >= static_cast<size_t>(expectedSize)) {
          memcpy(buffer, m_uyvyData.data(), expectedSize);
          videoBuffer->EndAccess(bmdBufferAccessWrite);
          videoBuffer->Release();
          return true;
        }
      }
      videoBuffer->EndAccess(bmdBufferAccessWrite);
    }
    videoBuffer->Release();
  }

  return false;
}

void MultiViewSource::fillQImage(QImage &image) {
  compositeFrame();

  if (image.size() != m_compositedImage.size()) {
    image = m_compositedImage.scaled(image.size(), Qt::KeepAspectRatio,
                                     Qt::SmoothTransformation);
  } else {
    image = m_compositedImage;
  }
}

} // namespace Monitor3G
