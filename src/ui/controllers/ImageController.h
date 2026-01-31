#ifndef IMAGECONTROLLER_H
#define IMAGECONTROLLER_H

#include <QComboBox>
#include <QHBoxLayout>
#include <QLabel>
#include <QWidget>

namespace Monitor3G {

class ImageController : public QWidget {
  Q_OBJECT

public:
  explicit ImageController(QWidget *parent = nullptr);

signals:
  void scalingModeChanged(int mode); // 0: Fit, 1: Fill

private slots:
  void onScalingChanged(int index);

private:
  void setupUI();

  QComboBox *m_scalingCombo;
};

} // namespace Monitor3G

#endif // IMAGECONTROLLER_H
