#include "SourceController.h"
#include "controllers/ImageController.h"
#include "controllers/ScreenCaptureController.h"
#include "controllers/VideoController.h"
#include <QLabel>
#include <QPropertyAnimation>

namespace Monitor3G {

SourceController::SourceController(QWidget *parent)
    : QWidget(parent), m_stack(nullptr) {
  setupUI();
}

void SourceController::setupUI() {
  QVBoxLayout *layout = new QVBoxLayout(this);
  layout->setContentsMargins(0, 0, 0, 0);

  // Title Bar
  QLabel *title = new QLabel("SOURCE SETTINGS", this);
  title->setStyleSheet("background-color: #3d3d3d; color: #ddd; padding: 5px; "
                       "font-weight: bold;");
  layout->addWidget(title);

  m_stack = new QStackedWidget(this);
  layout->addWidget(m_stack);

  // Placeholder
  m_placeholderLabel = new QLabel("Select a source to view settings", this);
  m_placeholderLabel->setAlignment(Qt::AlignCenter);
  m_placeholderLabel->setStyleSheet("color: #888;");
  m_stack->addWidget(m_placeholderLabel);

  // Initialize Empty Controllers (Placeholders for now)

  // Video
  m_videoController = new VideoController(this);
  m_stack->addWidget(m_videoController);

  // Image
  m_imageController = new ImageController(this);
  m_stack->addWidget(m_imageController);

  // Screen
  m_screenController = new ScreenCaptureController(this);
  m_stack->addWidget(m_screenController);

  // Default view
  m_stack->setCurrentWidget(m_placeholderLabel);
}

void SourceController::showController(ControllerType type) {
  QWidget *targetWidget = m_placeholderLabel;
  switch (type) {
  case None:
    targetWidget = m_placeholderLabel;
    break;
  case Video:
    targetWidget = m_videoController;
    break;
  case Image:
    targetWidget = m_imageController;
    break;
  case ScreenCapture:
    targetWidget = m_screenController;
    break;
  case LiveCamera:
    // Reusing placeholder or standard video controls later
    targetWidget = m_placeholderLabel;
    break;
  }

  if (m_stack->currentWidget() != targetWidget) {
    animateWidget(targetWidget);
    m_stack->setCurrentWidget(targetWidget);
  }
}

void SourceController::animateWidget(QWidget *widget) {
    QPropertyAnimation *animation = new QPropertyAnimation(widget, "windowOpacity");
    animation->setDuration(250);
    animation->setStartValue(0.0);
    animation->setEndValue(1.0);
    animation->setEasingCurve(QEasingCurve::InOutQuad);
    animation->start(QAbstractAnimation::DeleteWhenStopped);
}

} // namespace Monitor3G
