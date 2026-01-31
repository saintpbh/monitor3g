#ifndef SOURCECONTROLLER_H
#define SOURCECONTROLLER_H

#include <QLabel>
#include <QStackedWidget>
#include <QVBoxLayout>
#include <QWidget>

namespace Monitor3G {

class VideoController;
class ImageController;
class ScreenCaptureController;

class SourceController : public QWidget {
  Q_OBJECT

public:
  explicit SourceController(QWidget *parent = nullptr);

  enum ControllerType { None, Video, Image, ScreenCapture, LiveCamera };

  void showController(ControllerType type);

  // Specific Controllers
  VideoController *getVideoController() const { return m_videoController; }
  ImageController *getImageController() const { return m_imageController; }
  ScreenCaptureController *getScreenController() const {
    return m_screenController;
  }

private:
  void setupUI();
  void animateWidget(QWidget *widget);

  QStackedWidget *m_stack;
  QLabel *m_placeholderLabel;

  VideoController *m_videoController;
  ImageController *m_imageController;
  ScreenCaptureController *m_screenController;
};

} // namespace Monitor3G

#endif // SOURCECONTROLLER_H
