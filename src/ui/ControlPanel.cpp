#include "ControlPanel.h"
#include <QGroupBox>
#include <QHBoxLayout>
#include <QVBoxLayout>

namespace Monitor3G {

ControlPanel::ControlPanel(QWidget *parent)
    : QWidget(parent), m_isOutputting(false) {
  setupUI();
}

void ControlPanel::setupUI() {
  QVBoxLayout *mainLayout = new QVBoxLayout(this);
  mainLayout->setContentsMargins(8, 8, 8, 8);

  // Output Settings Group
  QGroupBox *settingsGroup = new QGroupBox(tr("Output Settings"), this);
  QHBoxLayout *settingsLayout = new QHBoxLayout(settingsGroup);

  // Device selection
  QLabel *deviceLabel = new QLabel(tr("Device:"), this);
  settingsLayout->addWidget(deviceLabel);

  m_deviceCombo = new QComboBox(this);
  m_deviceCombo->addItem(tr("Auto-detect"));
  settingsLayout->addWidget(m_deviceCombo, 1);

  // Format selection
  QLabel *formatLabel = new QLabel(tr("Format:"), this);
  settingsLayout->addWidget(formatLabel);

  m_formatCombo = new QComboBox(this);
  m_formatCombo->addItem(tr("1920x1080p 60fps"));
  m_formatCombo->addItem(tr("1920x1080i 60fps"));
  m_formatCombo->addItem(tr("1920x1080p 30fps"));
  m_formatCombo->addItem(tr("1920x1080i 50fps"));
  m_formatCombo->addItem(tr("1280x720p 60fps"));
  m_formatCombo->addItem(tr("1280x720p 50fps"));
  connect(m_formatCombo, QOverload<int>::of(&QComboBox::currentIndexChanged),
          this, &ControlPanel::onFormatChanged);
  settingsLayout->addWidget(m_formatCombo, 1);

  mainLayout->addWidget(settingsGroup);

  // Control Group
  QGroupBox *controlGroup = new QGroupBox(tr("Control"), this);
  QHBoxLayout *controlLayout = new QHBoxLayout(controlGroup);

  // Start/Stop button
  m_startStopBtn = new QPushButton(tr("▶ Start Output"), this);
  m_startStopBtn->setMinimumHeight(40);
  connect(m_startStopBtn, &QPushButton::clicked, this,
          &ControlPanel::onStartStopClicked);
  controlLayout->addWidget(m_startStopBtn);

  // Status label
  m_statusLabel = new QLabel(tr("Ready"), this);
  m_statusLabel->setStyleSheet("font-weight: bold;");
  controlLayout->addWidget(m_statusLabel);

  // Frame count
  m_frameCountLabel = new QLabel(tr("Frames: 0"), this);
  controlLayout->addWidget(m_frameCountLabel);

  controlLayout->addStretch();

  mainLayout->addWidget(controlGroup);
}

void ControlPanel::onStartStopClicked() {
  if (m_isOutputting) {
    emit stopOutputRequested();
    updateButtonState(false);
  } else {
    emit startOutputRequested();
    updateButtonState(true);
  }
}

void ControlPanel::onFormatChanged(int index) {
  emit outputFormatChanged(index);
}

void ControlPanel::updateButtonState(bool isOutputting) {
  m_isOutputting = isOutputting;

  if (isOutputting) {
    m_startStopBtn->setText(tr("⏸ Stop Output"));
    m_startStopBtn->setStyleSheet("background-color: #d32f2f; color: white;");
    m_statusLabel->setText(tr("● LIVE"));
    m_statusLabel->setStyleSheet("color: #4caf50; font-weight: bold;");
  } else {
    m_startStopBtn->setText(tr("▶ Start Output"));
    m_startStopBtn->setStyleSheet("");
    m_statusLabel->setText(tr("Ready"));
    m_statusLabel->setStyleSheet("font-weight: bold;");
  }
}

} // namespace Monitor3G
