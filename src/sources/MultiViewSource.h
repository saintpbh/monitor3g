#ifndef MULTIVIEWSOURCE_H
#define MULTIVIEWSOURCE_H

#include "AbstractSource.h"
#include <QImage>
#include <QMutex>
#include <memory>
#include <vector>

namespace Monitor3G {

// Layout presets for multi-view composition
enum class MultiViewLayout {
  PictureInPicture, // Main source + small overlay
  SideBySide,       // Two sources side by side
  QuadView,         // Four sources in 2x2 grid
  StackedVertical   // Two sources stacked vertically
};

class MultiViewSource : public AbstractSource {
  Q_OBJECT

public:
  explicit MultiViewSource(QObject *parent = nullptr);
  ~MultiViewSource() override;

  // AbstractSource interface
  SourceType getType() const override { return SourceType::MultiView; }
  QString getName() const override { return m_name; }
  bool start() override;
  void stop() override;
  bool isActive() const override { return m_isActive; }
  bool fillNextFrame(IDeckLinkMutableVideoFrame *frame) override;
  void fillQImage(QImage &image) override;

  // Multi-view specific
  void setLayout(MultiViewLayout layout);
  MultiViewLayout layout() const { return m_layout; }

  // Source management (up to 4 sources)
  void setMainSource(std::shared_ptr<AbstractSource> source);
  void setOverlaySource(std::shared_ptr<AbstractSource> source, int index = 0);
  void setSource(int slot, std::shared_ptr<AbstractSource> source);
  void clearSources();

  // PiP settings
  void setPipPosition(int x, int y);      // Position of PiP overlay
  void setPipSize(int width, int height); // Size of PiP overlay (default: 25%)

private:
  void compositeFrame();
  void convertToUYVY();
  void drawSourceToCanvas(const QImage &source, QImage &canvas, int x, int y,
                          int w, int h);

  QString m_name;
  bool m_isActive;
  MultiViewLayout m_layout;
  QMutex m_mutex;

  // Sub-sources (max 4)
  std::shared_ptr<AbstractSource> m_sources[4];

  // Composited output
  QImage m_compositedImage;
  std::vector<uint8_t> m_uyvyData;

  // PiP settings
  int m_pipX, m_pipY;
  int m_pipWidth, m_pipHeight;

  static constexpr int TARGET_WIDTH = 1920;
  static constexpr int TARGET_HEIGHT = 1080;
};

} // namespace Monitor3G

#endif // MULTIVIEWSOURCE_H
