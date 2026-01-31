#include "ScreenCaptureController.h"

namespace Monitor3G {

ScreenCaptureController::ScreenCaptureController(QWidget *parent)
    : QWidget(parent) {
  setupUI();
}

void ScreenCaptureController::setupUI() {
  QHBoxLayout *layout = new QHBoxLayout(this);
  layout->setContentsMargins(10, 10, 10, 10);
  layout->setSpacing(10);

  layout->addWidget(new QLabel("Display:", this));

  m_screenCombo = new QComboBox(this);
  m_screenCombo->setStyleSheet(
      "QComboBox { background-color: #555; color: #fff; border: 1px solid "
      "#777; border-radius: 4px; padding: 4px; }"
      "QComboBox::drop-down { border: none; }");
  connect(m_screenCombo, QOverload<int>::of(&QComboBox::currentIndexChanged),
          this, &ScreenCaptureController::onScreenChanged);
  layout->addWidget(m_screenCombo);

  m_cursorCheck = new QCheckBox("Show Cursor", this);
  m_cursorCheck->setStyleSheet("color: #ddd;");
  m_cursorCheck->setChecked(true);
  connect(m_cursorCheck, &QCheckBox::toggled, this,
          &ScreenCaptureController::onCursorToggled);
  layout->addWidget(m_cursorCheck);

  layout->addStretch();
}

void ScreenCaptureController::setScreenList(const QStringList &screens) {
  m_screenCombo->clear();
  m_screenCombo->addItems(screens);
}

void ScreenCaptureController::onScreenChanged(int index) {
  emit screenChanged(index);
}

void ScreenCaptureController::onCursorToggled(bool checked) {
  emit cursorVisibilityChanged(checked);
}

} // namespace Monitor3G
