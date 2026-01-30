#ifndef PREVIEWWIDGET_H
#define PREVIEWWIDGET_H

#include <QLabel>
#include <QWidget>

namespace Monitor3G {

class PreviewWidget : public QWidget {
  Q_OBJECT

public:
  explicit PreviewWidget(QWidget *parent = nullptr);

  void setFrame(const QImage &frame);
  void clear();

protected:
  void paintEvent(QPaintEvent *event) override;
  void resizeEvent(QResizeEvent *event) override;

private:
  void setupUI();
  void drawNoSignal(QPainter &painter);

  QImage m_currentFrame;
  bool m_hasFrame;
};

} // namespace Monitor3G

#endif // PREVIEWWIDGET_H
