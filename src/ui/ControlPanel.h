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

private slots:
  void onStartStopClicked();
  void onFormatChanged(int index);

private:
  void setupUI();
  void updateButtonState(bool isOutputting);

  QComboBox *m_formatCombo;
  QComboBox *m_deviceCombo;
  QPushButton *m_startStopBtn;
  QLabel *m_statusLabel;
  QLabel *m_frameCountLabel;

  bool m_isOutputting;
};

} // namespace Monitor3G

#endif // CONTROLPANEL_H
