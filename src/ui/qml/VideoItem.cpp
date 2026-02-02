#include "VideoItem.h"
#include "FrameProvider.h"

namespace Monitor3G {

VideoItem::VideoItem(QQuickItem *parent)
    : QQuickPaintedItem(parent), m_hasFrame(false) {
  // Connect to the singleton FrameProvider
  connect(&FrameProvider::instance(), &FrameProvider::frameReceived, this,
          &VideoItem::updateFrame);
}

void VideoItem::paint(QPainter *painter) {
  if (m_hasFrame && !m_currentFrame.isNull()) {
    QRectF targetRect(0, 0, width(), height());

    // Scale while keeping aspect ratio (mimics Qt::KeepAspectRatio)
    QSize scaledSize = m_currentFrame.size().scaled(QSize(width(), height()),
                                                    Qt::KeepAspectRatio);

    // Center the image
    double x = (width() - scaledSize.width()) / 2.0;
    double y = (height() - scaledSize.height()) / 2.0;
    QRectF drawRect(x, y, scaledSize.width(), scaledSize.height());

    painter->setRenderHint(QPainter::Antialiasing);
    painter->setRenderHint(QPainter::SmoothPixmapTransform);
    painter->drawImage(drawRect, m_currentFrame);
  } else {
    // Draw black background
    painter->fillRect(boundingRect(), Qt::black);
  }
}

bool VideoItem::hasFrame() const { return m_hasFrame; }

void VideoItem::updateFrame(const QImage &frame) {
  m_currentFrame = frame;
  if (!m_hasFrame) {
    m_hasFrame = true;
    emit hasFrameChanged();
  }
  update(); // Request a repaint
}

} // namespace Monitor3G
