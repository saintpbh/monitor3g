#ifndef VIDEOITEM_H
#define VIDEOITEM_H

#include <QImage>
#include <QPainter>
#include <QQuickPaintedItem>

namespace Monitor3G {

class VideoItem : public QQuickPaintedItem {
  Q_OBJECT
  Q_PROPERTY(bool hasFrame READ hasFrame NOTIFY hasFrameChanged)

public:
  explicit VideoItem(QQuickItem *parent = nullptr);
  void paint(QPainter *painter) override;

  bool hasFrame() const;

public slots:
  void updateFrame(const QImage &frame);

signals:
  void hasFrameChanged();

private:
  QImage m_currentFrame;
  bool m_hasFrame;
};

} // namespace Monitor3G

#endif // VIDEOITEM_H
