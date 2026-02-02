#ifndef VIDEOFILESOURCE_H
#define VIDEOFILESOURCE_H

#include "AbstractSource.h"
#include <QImage>
#include <QMutex>
#include <QQueue>
#include <QThread>
#include <atomic>
#include <condition_variable>
#include <queue>

extern "C" {
#include <libavcodec/avcodec.h>
#include <libavformat/avformat.h>
#include <libavutil/imgutils.h>
#include <libswscale/swscale.h>
}

namespace Monitor3G {

// Structure to hold a decoded video frame
struct VideoFrame {
  QByteArray data; // UYVY data
  double pts;
  long width;
  long height;
};

class VideoFileSource : public AbstractSource {
  Q_OBJECT

public:
  explicit VideoFileSource(QObject *parent = nullptr);
  ~VideoFileSource() override;

  // AbstractSource interface
  SourceType getType() const override { return SourceType::VideoFile; }
  QString getName() const override { return "Video File"; }
  bool start() override;
  void stop() override;
  bool isActive() const override { return m_isActive; }
  bool fillNextFrame(IDeckLinkMutableVideoFrame *frame) override;
  void fillQImage(QImage &image) override;

  // File handling
  bool openFile(const QString &filePath);
  void closeFile();

  // Playback control
  void play();
  void pause();
  void seek(int64_t timestamp);
  bool isPaused() const { return m_isPaused; }
  void setLoop(bool loop) { m_isLooping = loop; }
  bool isLooping() const { return m_isLooping; }
  int64_t getDuration() const;
  int64_t getPosition() const;

private:
  void decodeLoop(); // Worker thread function

  QString m_filePath;
  std::atomic<bool> m_isActive;
  std::atomic<bool> m_isPaused;
  std::atomic<bool> m_isLooping;
  std::atomic<int64_t> m_positionMs;

  // FFmpeg
  AVFormatContext *m_formatCtx = nullptr;
  AVCodecContext *m_codecCtx = nullptr;
  SwsContext *m_swsCtx = nullptr;
  SwsContext *m_previewSwsCtx = nullptr;
  int m_videoStreamIndex = -1;

  // Frame Queue
  std::queue<VideoFrame> m_frameQueue;
  const size_t MAX_QUEUE_SIZE = 30;
  QMutex m_queueMutex;
  std::condition_variable m_queueCond;

  // Threading
  std::thread m_decodeThread;
  std::atomic<bool> m_stopDecode;

  // Preview
  QImage m_currentPreview;
  QMutex m_previewMutex;
};

} // namespace Monitor3G

#endif // VIDEOFILESOURCE_H
