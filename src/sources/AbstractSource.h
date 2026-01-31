#ifndef ABSTRACTSOURCE_H
#define ABSTRACTSOURCE_H

#include "DeckLinkAPI.h"
#include <QImage>
#include <QObject>
#include <QString>

namespace Monitor3G {

enum class SourceType {
  TestPattern,
  VideoFile,
  Image,
  LiveCamera,
  ScreenCapture,
  PDF,
  MultiView
};

class AbstractSource : public QObject {
  Q_OBJECT

public:
  explicit AbstractSource(QObject *parent = nullptr) : QObject(parent) {}
  virtual ~AbstractSource() = default;

  // Metadata
  virtual SourceType getType() const = 0;
  virtual QString getName() const = 0;

  // Control
  virtual bool start() = 0;
  virtual void stop() = 0;
  virtual bool isActive() const = 0;

  // Frame Generation
  // Returns true if frame was filled, false otherwise
  virtual bool fillNextFrame(IDeckLinkMutableVideoFrame *frame) = 0;

  // Preview Generation (for UI)
  virtual void fillQImage(QImage &image) = 0;

signals:
  void started();
  void stopped();
  void errorOccurred(const QString &message);
};

} // namespace Monitor3G

#endif // ABSTRACTSOURCE_H
