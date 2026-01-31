#include "ImageController.h"

namespace Monitor3G {

ImageController::ImageController(QWidget *parent) : QWidget(parent) {
  setupUI();
}

void ImageController::setupUI() {
  QHBoxLayout *layout = new QHBoxLayout(this);
  layout->setContentsMargins(10, 10, 10, 10);
  layout->setSpacing(10);

  layout->addWidget(new QLabel("Scaling:", this));

  m_scalingCombo = new QComboBox(this);
  m_scalingCombo->addItem("Fit to Screen", 0);
  m_scalingCombo->addItem("Fill Screen", 1);

  // Style the combo box
  m_scalingCombo->setStyleSheet(
      "QComboBox { background-color: #555; color: #fff; border: 1px solid "
      "#777; border-radius: 4px; padding: 4px; }"
      "QComboBox::drop-down { border: none; }");

  connect(m_scalingCombo, QOverload<int>::of(&QComboBox::currentIndexChanged),
          this, &ImageController::onScalingChanged);

  layout->addWidget(m_scalingCombo);
  layout->addStretch();
}

void ImageController::onScalingChanged(int index) {
  emit scalingModeChanged(index);
}

} // namespace Monitor3G
