#ifndef FULLSCREENPREVIEW_H
#define FULLSCREENPREVIEW_H

#include <QLabel>
#include <QScreen>
#include <QWidget>

namespace Monitor3G {

class FullscreenPreview : public QWidget {
  Q_OBJECT

public:
  explicit FullscreenPreview(QWidget *parent = nullptr);
  ~FullscreenPreview() override;

  // Show fullscreen on specified screen (0 = primary, 1+ = secondary)
  void showOnScreen(int screenIndex);
  void showOnScreen(QScreen *screen);

  // Update the preview frame
  void updateFrame(const QImage &frame);

  // Toggle visibility
  void toggleFullscreen();

public slots:
  void onPreviewFrameReady(const QImage &frame);

protected:
  void keyPressEvent(QKeyEvent *event) override;
  void mouseDoubleClickEvent(QMouseEvent *event) override;

private:
  void setupUI();

  QLabel *m_displayLabel;
  QImage m_currentFrame;
  bool m_isFullscreen;
};

} // namespace Monitor3G

#endif // FULLSCREENPREVIEW_H
