#include "VideoController.h"
#include <QStyle>

namespace Monitor3G {

VideoController::VideoController(QWidget *parent)
    : QWidget(parent), m_isPlaying(false), m_durationMs(0) {
  setupUI();
}

void VideoController::setupUI() {
  QVBoxLayout *mainLayout = new QVBoxLayout(this);
  mainLayout->setContentsMargins(10, 10, 10, 10);
  mainLayout->setSpacing(10);

  // Time Slider
  m_seekSlider = new QSlider(Qt::Horizontal, this);
  m_seekSlider->setRange(0, 0);
  connect(m_seekSlider, &QSlider::sliderReleased, this,
          &VideoController::onSliderReleased);
  mainLayout->addWidget(m_seekSlider);

  // Controls Row
  QHBoxLayout *controlsLayout = new QHBoxLayout();

  m_playPauseBtn = new QPushButton("Play", this);
  // Use standard icons if available, or text
  m_playPauseBtn->setIcon(style()->standardIcon(QStyle::SP_MediaPlay));
  connect(m_playPauseBtn, &QPushButton::clicked, this,
          &VideoController::onPlayPauseClicked);
  controlsLayout->addWidget(m_playPauseBtn);

  m_stopBtn = new QPushButton(this);
  m_stopBtn->setIcon(style()->standardIcon(QStyle::SP_MediaStop));
  connect(m_stopBtn, &QPushButton::clicked, this,
          [this]() { emit stopRequested(); });
  controlsLayout->addWidget(m_stopBtn);

  m_timeLabel = new QLabel("00:00 / 00:00", this);
  m_timeLabel->setStyleSheet("color: #ddd; font-family: monospace;");
  controlsLayout->addWidget(m_timeLabel);

  controlsLayout->addStretch();

  m_loopCheck = new QCheckBox("Loop", this);
  m_loopCheck->setStyleSheet("color: #ddd;");
  connect(m_loopCheck, &QCheckBox::toggled, this,
          &VideoController::onLoopToggled);
  controlsLayout->addWidget(m_loopCheck);

  mainLayout->addLayout(controlsLayout);
  mainLayout->addStretch(); // Push everything up
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
  m_playPauseBtn->setText(isPlaying ? "Pause" : "Play");
  m_playPauseBtn->setIcon(style()->standardIcon(
      isPlaying ? QStyle::SP_MediaPause : QStyle::SP_MediaPlay));
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
