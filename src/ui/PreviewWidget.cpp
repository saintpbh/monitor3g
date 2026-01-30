#include "PreviewWidget.h"
#include <QPaintEvent>
#include <QPainter>

namespace Monitor3G {

PreviewWidget::PreviewWidget(QWidget *parent)
    : QWidget(parent), m_hasFrame(false) {
  setupUI();
}

void PreviewWidget::setupUI() {
  setMinimumSize(640, 360);
  setStyleSheet("background-color: #1a1a1a;");
}

void PreviewWidget::setFrame(const QImage &frame) {
  m_currentFrame = frame;
  m_hasFrame = true;
  update();
}

void PreviewWidget::clear() {
  m_currentFrame = QImage();
  m_hasFrame = false;
  update();
}

void PreviewWidget::paintEvent(QPaintEvent *event) {
  QPainter painter(this);
  painter.setRenderHint(QPainter::Antialiasing);

  if (m_hasFrame && !m_currentFrame.isNull()) {
    // Scale frame to fit widget while maintaining aspect ratio
    QSize scaledSize =
        m_currentFrame.size().scaled(size(), Qt::KeepAspectRatio);
    QRect targetRect((width() - scaledSize.width()) / 2,
                     (height() - scaledSize.height()) / 2, scaledSize.width(),
                     scaledSize.height());

    painter.drawImage(targetRect, m_currentFrame);
  } else {
    drawNoSignal(painter);
  }
}

void PreviewWidget::resizeEvent(QResizeEvent *event) {
  QWidget::resizeEvent(event);
  update();
}

void PreviewWidget::drawNoSignal(QPainter &painter) {
  // Draw "No Signal" placeholder
  painter.fillRect(rect(), QColor(26, 26, 26));

  painter.setPen(QColor(100, 100, 100));
  QFont font = painter.font();
  font.setPointSize(24);
  painter.setFont(font);

  painter.drawText(rect(), Qt::AlignCenter,
                   tr("No Signal\n\nSelect a source to begin"));
}

} // namespace Monitor3G
