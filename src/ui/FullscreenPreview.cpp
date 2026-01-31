#include "FullscreenPreview.h"
#include <QApplication>
#include <QKeyEvent>
#include <QVBoxLayout>

namespace Monitor3G {

FullscreenPreview::FullscreenPreview(QWidget *parent)
    : QWidget(parent, Qt::Window), m_isFullscreen(false) {
  setupUI();
  setWindowTitle("Fullscreen Preview");
  setStyleSheet("background-color: black;");

  // Hide by default
  hide();
}

FullscreenPreview::~FullscreenPreview() = default;

void FullscreenPreview::setupUI() {
  auto *layout = new QVBoxLayout(this);
  layout->setContentsMargins(0, 0, 0, 0);
  layout->setSpacing(0);

  m_displayLabel = new QLabel(this);
  m_displayLabel->setAlignment(Qt::AlignCenter);
  m_displayLabel->setStyleSheet("background-color: black;");
  m_displayLabel->setScaledContents(false);
  m_displayLabel->setSizePolicy(QSizePolicy::Expanding, QSizePolicy::Expanding);

  layout->addWidget(m_displayLabel);
}

void FullscreenPreview::showOnScreen(int screenIndex) {
  auto screens = QApplication::screens();
  if (screenIndex >= 0 && screenIndex < screens.size()) {
    showOnScreen(screens[screenIndex]);
  } else if (!screens.isEmpty()) {
    // Default to primary or last screen
    showOnScreen(screens.last());
  }
}

void FullscreenPreview::showOnScreen(QScreen *screen) {
  if (!screen)
    return;

  // Move to screen and show fullscreen
  setScreen(screen);
  move(screen->geometry().topLeft());
  showFullScreen();
  m_isFullscreen = true;
}

void FullscreenPreview::updateFrame(const QImage &frame) {
  if (frame.isNull())
    return;

  m_currentFrame = frame;

  // Scale to fit the display while maintaining aspect ratio
  QSize displaySize = m_displayLabel->size();
  QPixmap scaled = QPixmap::fromImage(frame).scaled(
      displaySize, Qt::KeepAspectRatio, Qt::SmoothTransformation);
  m_displayLabel->setPixmap(scaled);
}

void FullscreenPreview::toggleFullscreen() {
  if (isVisible()) {
    hide();
    m_isFullscreen = false;
  } else {
    // Show on secondary screen if available, otherwise primary
    auto screens = QApplication::screens();
    if (screens.size() > 1) {
      showOnScreen(1); // Secondary screen
    } else {
      showOnScreen(0); // Primary screen
    }
  }
}

void FullscreenPreview::onPreviewFrameReady(const QImage &frame) {
  if (isVisible()) {
    updateFrame(frame);
  }
}

void FullscreenPreview::keyPressEvent(QKeyEvent *event) {
  switch (event->key()) {
  case Qt::Key_Escape:
  case Qt::Key_F11:
    // Close fullscreen on Escape or F11
    hide();
    m_isFullscreen = false;
    break;
  default:
    QWidget::keyPressEvent(event);
    break;
  }
}

void FullscreenPreview::mouseDoubleClickEvent(QMouseEvent *event) {
  // Double-click to exit fullscreen
  Q_UNUSED(event);
  hide();
  m_isFullscreen = false;
}

} // namespace Monitor3G
