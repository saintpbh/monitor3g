#ifndef SCREENCAPTURECONTROLLER_H
#define SCREENCAPTURECONTROLLER_H

#include <QCheckBox>
#include <QComboBox>
#include <QHBoxLayout>
#include <QLabel>
#include <QWidget>

namespace Monitor3G {

class ScreenCaptureController : public QWidget {
  Q_OBJECT

public:
  explicit ScreenCaptureController(QWidget *parent = nullptr);

  void setScreenList(const QStringList &screens);

signals:
  void screenChanged(int index);
  void cursorVisibilityChanged(bool visible);

private slots:
  void onScreenChanged(int index);
  void onCursorToggled(bool checked);

private:
  void setupUI();

  QComboBox *m_screenCombo;
  QCheckBox *m_cursorCheck;
};

} // namespace Monitor3G

#endif // SCREENCAPTURECONTROLLER_H
