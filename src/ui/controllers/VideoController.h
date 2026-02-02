#ifndef VIDEOCONTROLLER_H
#define VIDEOCONTROLLER_H

#include <QCheckBox>
#include <QHBoxLayout>
#include <QLabel>
#include <QPushButton>
#include <QSlider>
#include <QVBoxLayout>
#include <QWidget>

namespace Monitor3G {

class VideoController : public QWidget {
  Q_OBJECT

public:
  explicit VideoController(QWidget *parent = nullptr);

  void updateTime(int64_t currentMs, int64_t totalMs);
  void setPlaying(bool isPlaying);

signals:
  void playRequested();
  void pauseRequested();
  void stopRequested();
  void rewindRequested();
  void fastForwardRequested();
  void seekRequested(int64_t positionMs);
  void loopChanged(bool enabled);

private slots:
  void onPlayPauseClicked();
  void onSliderReleased();
  void onLoopToggled(bool checked);

private:
  void setupUI();
  QString formatTime(int64_t ms);

  QPushButton *m_playPauseBtn;
  QPushButton *m_stopBtn;
  QPushButton *m_rewindBtn;
  QPushButton *m_fastForwardBtn;
  QSlider *m_seekSlider;
  QLabel *m_timeLabel; // "00:00 / 00:00"
  QCheckBox *m_loopCheck;

  bool m_isPlaying;
  int64_t m_durationMs;
};

} // namespace Monitor3G

#endif // VIDEOCONTROLLER_H
