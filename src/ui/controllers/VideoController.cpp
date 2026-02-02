#include "VideoController.h"
#include <QIcon>
#include <QStyle>

namespace Monitor3G {

VideoController::VideoController(QWidget *parent)
    : QWidget(parent), m_isPlaying(false), m_durationMs(0) {
  setupUI();
}

void VideoController::setupUI() {
  QVBoxLayout *mainLayout = new QVBoxLayout(this);
  mainLayout->setContentsMargins(20, 20, 20, 20);
  mainLayout->setSpacing(15);

  // Apply Hardware Style QSS
  QString buttonStyle = R"(
    QPushButton {
      background-color: #2b2b2b;
      border: 2px solid #3e3e3e;
      border-radius: 8px;
      color: #e0e0e0;
      font-size: 18px;
      padding: 12px 24px;
      min-width: 60px;
    }
    QPushButton:hover {
      background-color: #383838;
      border-color: #505050;
    }
    QPushButton:pressed {
      background-color: #1a1a1a;
      border-color: #007acc;
      color: #007acc;
    }
  )";
  this->setStyleSheet(buttonStyle);

  // Time Slider (Seek Bar)
  m_seekSlider = new QSlider(Qt::Horizontal, this);
  m_seekSlider->setRange(0, 0);
  m_seekSlider->setStyleSheet(R"(
      QSlider::groove:horizontal {
          border: 1px solid #3d3d3d;
          height: 8px;
          background: #202020;
          margin: 2px 0;
          border-radius: 4px;
      }
      QSlider::handle:horizontal {
          background: #007acc;
          border: 1px solid #007acc;
          width: 18px;
          height: 18px;
          margin: -7px 0;
          border-radius: 9px;
      }
  )");
  connect(m_seekSlider, &QSlider::sliderReleased, this,
          &VideoController::onSliderReleased);
  mainLayout->addWidget(m_seekSlider);

  // Transport Controls Layout (Center)
  QHBoxLayout *controlsLayout = new QHBoxLayout();
  controlsLayout->setAlignment(Qt::AlignCenter);
  controlsLayout->setSpacing(20);

  // Rewind
  m_rewindBtn = new QPushButton("⏪", this);
  m_rewindBtn->setToolTip("Rewind 5s");
  connect(m_rewindBtn, &QPushButton::clicked, this,
          [this]() { emit rewindRequested(); });
  controlsLayout->addWidget(m_rewindBtn);

  // Stop
  m_stopBtn = new QPushButton("⏹", this);
  connect(m_stopBtn, &QPushButton::clicked, this,
          [this]() { emit stopRequested(); });
  controlsLayout->addWidget(m_stopBtn);

  // Play/Pause (Big)
  m_playPauseBtn = new QPushButton("▶", this);
  m_playPauseBtn->setStyleSheet(buttonStyle +
                                "QPushButton { font-size: 24px; min-width: "
                                "80px; background-color: #333; }");
  connect(m_playPauseBtn, &QPushButton::clicked, this,
          &VideoController::onPlayPauseClicked);
  controlsLayout->addWidget(m_playPauseBtn);

  // Fast Forward
  m_fastForwardBtn = new QPushButton("⏩", this);
  m_fastForwardBtn->setToolTip("Forward 5s");
  connect(m_fastForwardBtn, &QPushButton::clicked, this,
          [this]() { emit fastForwardRequested(); });
  controlsLayout->addWidget(m_fastForwardBtn);

  mainLayout->addLayout(controlsLayout);

  // Bottom Row: Time and Loop
  QHBoxLayout *bottomLayout = new QHBoxLayout();

  m_timeLabel = new QLabel("00:00:00 / 00:00:00", this);
  m_timeLabel->setStyleSheet("color: #007acc; font-family: monospace; "
                             "font-size: 14px; font-weight: bold;");
  bottomLayout->addWidget(m_timeLabel);

  bottomLayout->addStretch();

  m_loopCheck = new QCheckBox("Loop Playback", this);
  m_loopCheck->setStyleSheet(
      "QCheckBox { color: #aaa; font-size: 14px; spacing: 5px; } "
      "QCheckBox::indicator { width: 18px; height: 18px; }");
  connect(m_loopCheck, &QCheckBox::toggled, this,
          &VideoController::onLoopToggled);
  bottomLayout->addWidget(m_loopCheck);

  mainLayout->addLayout(bottomLayout);
}

void VideoController::onPlayPauseClicked() {
  if (m_isPlaying) {
    emit pauseRequested();
  } else {
    emit playRequested();
  }
}

void VideoController::setPlaying(bool isPlaying) {
  m_isPlaying = isPlaying;
  m_playPauseBtn->setText(isPlaying ? "⏸" : "▶");
  // Remove icon setting to keep the clean text/symbol look
  m_playPauseBtn->setIcon(QIcon());
}

void VideoController::updateTime(int64_t currentMs, int64_t totalMs) {
  if (!m_seekSlider->isSliderDown()) {
    m_seekSlider->setMaximum(static_cast<int>(totalMs));
    m_seekSlider->setValue(static_cast<int>(currentMs));
  }
  m_durationMs = totalMs;
  m_timeLabel->setText(
      QString("%1 / %2").arg(formatTime(currentMs)).arg(formatTime(totalMs)));
}

void VideoController::onSliderReleased() {
  emit seekRequested(m_seekSlider->value());
}

void VideoController::onLoopToggled(bool checked) { emit loopChanged(checked); }

QString VideoController::formatTime(int64_t ms) {
  int seconds = (ms / 1000) % 60;
  int minutes = (ms / (1000 * 60)) % 60;
  int hours = (ms / (1000 * 60 * 60));

  if (hours > 0)
    return QString("%1:%2:%3")
        .arg(hours, 2, 10, QChar('0'))
        .arg(minutes, 2, 10, QChar('0'))
        .arg(seconds, 2, 10, QChar('0'));

  return QString("%1:%2")
      .arg(minutes, 2, 10, QChar('0'))
      .arg(seconds, 2, 10, QChar('0'));
}

} // namespace Monitor3G
