#include "ControlPanel.h"
#include <QFrame>
#include <QGroupBox>
#include <QHBoxLayout>
#include <QTime>
#include <QTimer>
#include <QVBoxLayout>

namespace Monitor3G {

ControlPanel::ControlPanel(QWidget *parent)
    : QWidget(parent), m_isOutputting(false), m_testPatternActive(false) {
  setupUI();
  m_updateTimer = new QTimer(this);
  connect(m_updateTimer, &QTimer::timeout, this, &ControlPanel::onTimerTick);
  m_updateTimer->start(100);
}

void ControlPanel::setupUI() {
  QVBoxLayout *mainLayout = new QVBoxLayout(this);
  mainLayout->setContentsMargins(4, 4, 4, 4);
  mainLayout->setSpacing(4);

  // --- Row 1: Hardware Configuration ---
  QFrame *configFrame = new QFrame(this);
  configFrame->setStyleSheet("background-color: #2b2b2b; border-radius: 4px;");
  QHBoxLayout *configLayout = new QHBoxLayout(configFrame);
  configLayout->setContentsMargins(8, 4, 8, 4);

  QLabel *configTitle = new QLabel("HARDWARE CONFIGURATION", this);
  configTitle->setStyleSheet(
      "color: #888; font-size: 10px; font-weight: bold;");

  QLabel *devLabel = new QLabel("INTERFACE:", this);
  devLabel->setStyleSheet("color: #ccc; font-weight: bold;");
  m_deviceCombo = new QComboBox(this);
  m_deviceCombo->addItem("ULTRASTUDIO MONITOR 3G #1");
  m_deviceCombo->setStyleSheet("background-color: #1a1a1a; padding: 2px;");

  QLabel *fmtLabel = new QLabel("FORMAT:", this);
  fmtLabel->setStyleSheet("color: #ccc; font-weight: bold;");
  m_formatCombo = new QComboBox(this);
  m_formatCombo->addItem("1080p 60");
  m_formatCombo->addItem("1080i 59.94");
  m_formatCombo->addItem("720p 60");
  m_formatCombo->setStyleSheet("background-color: #1a1a1a; padding: 2px;");
  connect(m_formatCombo, QOverload<int>::of(&QComboBox::currentIndexChanged),
          this, &ControlPanel::onFormatChanged);

  configLayout->addWidget(configTitle);
  configLayout->addSpacing(10);
  configLayout->addWidget(devLabel);
  configLayout->addWidget(m_deviceCombo, 1);
  configLayout->addSpacing(10);
  configLayout->addWidget(fmtLabel);
  configLayout->addWidget(m_formatCombo, 0);

  mainLayout->addWidget(configFrame);

  // --- Row 2: Audio Routing & Transport ---
  QHBoxLayout *midLayout = new QHBoxLayout();

  // Audio Routing (Left)
  QFrame *audioFrame = new QFrame(this);
  audioFrame->setStyleSheet("background-color: #2b2b2b; border-radius: 4px;");
  QVBoxLayout *audioLayout = new QVBoxLayout(audioFrame);
  audioLayout->setContentsMargins(8, 4, 8, 4);

  QLabel *audioTitle = new QLabel("AUDIO ROUTING", this);
  audioTitle->setStyleSheet("color: #888; font-size: 10px; font-weight: bold;");

  QHBoxLayout *audioInputLayout = new QHBoxLayout();
  QLabel *outLabel = new QLabel("OUTPUT:", this);
  m_audioOutputCombo = new QComboBox(this);
  m_audioOutputCombo->addItem(
      "MacBook Pro 스피커"); // Placeholder from screenshot
  m_audioOutputCombo->addItem("UltraStudio Monitor 3G");
  m_audioOutputCombo->setStyleSheet(
      "background-color: #1a1a1a; padding: 4px; min-height: 20px;");

  audioInputLayout->addWidget(outLabel);
  audioInputLayout->addWidget(m_audioOutputCombo, 1);

  audioLayout->addWidget(audioTitle);
  audioLayout->addLayout(audioInputLayout);

  // Transport (Right)
  QFrame *transportFrame = new QFrame(this);
  transportFrame->setStyleSheet(
      "background-color: #2b2b2b; border-radius: 4px;");
  QVBoxLayout *transportLayout = new QVBoxLayout(transportFrame);
  transportLayout->setContentsMargins(8, 4, 8, 4);

  QLabel *transportTitle = new QLabel("TRANSPORT", this);
  transportTitle->setStyleSheet(
      "color: #888; font-size: 10px; font-weight: bold;");

  QHBoxLayout *btnLayout = new QHBoxLayout();

  m_onAirBtn = new QPushButton("ON AIR", this);
  m_onAirBtn->setCheckable(true);
  m_onAirBtn->setMinimumSize(80, 30);
  m_onAirBtn->setStyleSheet(
      "QPushButton { background-color: #440000; color: #ffaaaa; border: 1px "
      "solid #660000; border-radius: 3px; font-weight: bold; }"
      "QPushButton:checked { background-color: #cc0000; color: white; border: "
      "1px solid #ff0000; }"
      "QPushButton:hover { background-color: #660000; }");
  connect(m_onAirBtn, &QPushButton::clicked, this,
          &ControlPanel::onStartStopClicked);

  m_testPatternBtn = new QPushButton("TEST\nPATTERN", this);
  m_testPatternBtn->setCheckable(true);
  m_testPatternBtn->setMinimumSize(80, 30);
  m_testPatternBtn->setStyleSheet(
      "QPushButton { background-color: #443300; color: #ffddaa; border: 1px "
      "solid #664400; border-radius: 3px; font-weight: bold; font-size: 9px; }"
      "QPushButton:checked { background-color: #ffaa00; color: black; border: "
      "1px solid #ffcc00; }"
      "QPushButton:hover { background-color: #664400; }");
  connect(m_testPatternBtn, &QPushButton::clicked, this,
          &ControlPanel::onTestPatternClicked);

  btnLayout->addWidget(m_onAirBtn);
  btnLayout->addWidget(m_testPatternBtn);

  transportLayout->addWidget(transportTitle);
  transportLayout->addLayout(btnLayout);

  midLayout->addWidget(audioFrame, 1);
  midLayout->addWidget(transportFrame, 0);

  mainLayout->addLayout(midLayout);

  // --- Row 3: Status Bar ---
  QHBoxLayout *statusLayout = new QHBoxLayout();

  QLabel *statusTitle = new QLabel("STATUS: ", this);
  statusTitle->setStyleSheet("color: #888; font-weight: bold;");

  m_statusLabel = new QLabel("IDLE", this);
  m_statusLabel->setStyleSheet("color: #ffcc00; font-weight: bold;");

  statusLayout->addWidget(statusTitle);
  statusLayout->addWidget(m_statusLabel);
  statusLayout->addStretch();

  QLabel *tcTitle = new QLabel("TIMECODE: ", this);
  tcTitle->setStyleSheet("color: #888;");
  m_timecodeLabel = new QLabel("00:00:00:00", this);
  m_timecodeLabel->setStyleSheet(
      "color: #00ff00; font-family: monospace; font-size: 14px; "
      "background-color: black; padding: 2px 6px; border-radius: 2px;");

  statusLayout->addWidget(tcTitle);
  statusLayout->addWidget(m_timecodeLabel);

  mainLayout->addLayout(statusLayout);
  mainLayout->addStretch();
}

void ControlPanel::onStartStopClicked() {
  bool active = m_onAirBtn->isChecked();
  if (active) {
    emit startOutputRequested();
  } else {
    emit stopOutputRequested();
  }
  updateButtonState(active);
}

void ControlPanel::onTestPatternClicked() {
  m_testPatternActive = m_testPatternBtn->isChecked();
  emit testPatternRequested(m_testPatternActive);
}

void ControlPanel::onFormatChanged(int index) {
  emit outputFormatChanged(index);
}

void ControlPanel::onTimerTick() {
  if (m_isOutputting) {
    // Mock Timecode
    QTime now = QTime::currentTime();
    m_timecodeLabel->setText(now.toString("HH:mm:ss:zzz").left(11));
  }
}

void ControlPanel::updateButtonState(bool isOutputting) {
  m_isOutputting = isOutputting;
  // Button state is handled by QCheckable logic mostly, but we update status
  // text
  if (isOutputting) {
    m_statusLabel->setText("ON AIR");
    m_statusLabel->setStyleSheet("color: #ff0000; font-weight: bold;");
    m_onAirBtn->setChecked(true); // Ensure sync
  } else {
    m_statusLabel->setText("IDLE");
    m_statusLabel->setStyleSheet("color: #ffcc00; font-weight: bold;");
    m_onAirBtn->setChecked(false);
  }
}

} // namespace Monitor3G
