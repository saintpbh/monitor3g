#ifndef CONTROLPANEL_H
#define CONTROLPANEL_H

#include <QComboBox>
#include <QLabel>
#include <QPushButton>
#include <QWidget>

namespace Monitor3G {

class ControlPanel : public QWidget {
  Q_OBJECT

public:
  explicit ControlPanel(QWidget *parent = nullptr);

signals:
  void outputFormatChanged(int formatIndex);
  void startOutputRequested();
  void stopOutputRequested();
  void testPatternRequested(bool active);

private slots:
  void onStartStopClicked();
  void onTestPatternClicked();
  void onFormatChanged(int index);
  void onTimerTick();

private:
  void setupUI();
  void updateButtonState(bool isOutputting);

  // Hardware Config
  QComboBox *m_deviceCombo;
  QComboBox *m_formatCombo;

  // Audio Routing
  QComboBox *m_audioOutputCombo;

  // Transport
  QPushButton *m_onAirBtn;
  QPushButton *m_testPatternBtn;

  // Status
  QLabel *m_statusLabel;
  QLabel *m_timecodeLabel;

  bool m_isOutputting;
  bool m_testPatternActive;
  QTimer *m_updateTimer;
};

} // namespace Monitor3G

#endif // CONTROLPANEL_H
