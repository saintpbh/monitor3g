#ifndef FRAMEPROVIDER_H
#define FRAMEPROVIDER_H

#include <QImage>
#include <QObject>
#include <mutex>

namespace Monitor3G {

class FrameProvider : public QObject {
  Q_OBJECT

public:
  static FrameProvider &instance() {
    static FrameProvider instance;
    return instance;
  }

  // Thread-safe way to deliver frames
  void deliverFrame(const QImage &frame) { emit frameReceived(frame); }
  void deliverPreviewFrame(const QImage &frame) {
    emit previewFrameReceived(frame);
  }
  void deliverProgramFrame(const QImage &frame) {
    emit programFrameReceived(frame);
  }

signals:
  void frameReceived(const QImage &frame); // Legacy, for backward compatibility
  void previewFrameReceived(const QImage &frame);
  void programFrameReceived(const QImage &frame);

private:
  FrameProvider() {}
  ~FrameProvider() {}
  FrameProvider(const FrameProvider &) = delete;
  FrameProvider &operator=(const FrameProvider &) = delete;
};

} // namespace Monitor3G

#endif // FRAMEPROVIDER_H
