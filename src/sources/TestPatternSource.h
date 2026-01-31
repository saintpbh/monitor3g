#ifndef TESTPATTERNSOURCE_H
#define TESTPATTERNSOURCE_H

#include "AbstractSource.h"
#include <QTimer>

namespace Monitor3G {

enum class TestPattern {
  ColorBars,   // SMPTE Color Bars
  BlackFrame,  // Solid black
  WhiteFrame,  // Solid white
  Checkerboard // Checkerboard pattern
};

class TestPatternSource : public AbstractSource {
  Q_OBJECT

public:
  explicit TestPatternSource(QObject *parent = nullptr);
  ~TestPatternSource() override;

  SourceType getType() const override { return SourceType::TestPattern; }
  QString getName() const override { return "Test Pattern"; }

  void setPattern(TestPattern pattern) { m_pattern = pattern; }
  TestPattern pattern() const { return m_pattern; }

  // AbstractSource interface
  bool start() override;
  void stop() override;
  bool isActive() const override { return m_isActive; }

  bool fillNextFrame(IDeckLinkMutableVideoFrame *frame) override;
  void fillQImage(QImage &image) override;

private:
  void generateColorBars(void *buffer, int width, int height);
  void generateBlackFrame(void *buffer, int width, int height);
  void generateWhiteFrame(void *buffer, int width, int height);
  void generateCheckerboard(void *buffer, int width, int height);

  void setYUV(uint8_t *buffer, uint8_t y, uint8_t u, uint8_t v);

  TestPattern m_pattern;
  QTimer *m_timer;
  bool m_isActive;
  int m_frameWidth;
  int m_frameHeight;
};

} // namespace Monitor3G

#endif // TESTPATTERNSOURCE_H
