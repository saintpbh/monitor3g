#include "VideoFileSource.h"
#include "../utils/Logger.h"
#include <QPainter>

namespace Monitor3G {

VideoFileSource::VideoFileSource(QObject *parent)
    : AbstractSource(parent), m_isActive(false), m_isPaused(false),
      m_stopDecode(false) {}

VideoFileSource::~VideoFileSource() {
  stop();
  closeFile();
}

bool VideoFileSource::start() {
  if (m_isActive)
    return true;

  if (m_filePath.isEmpty()) {
    LOG_ERROR("No file loaded");
    return false;
  }

  LOG_INFO("Starting VideoFileSource");
  m_isActive = true;

  // Clear queue
  {
    QMutexLocker locker(&m_queueMutex);
    std::queue<VideoFrame> empty;
    std::swap(m_frameQueue, empty);
  }

  // Start decode thread
  m_stopDecode = false;
  m_decodeThread = std::thread(&VideoFileSource::decodeLoop, this);

  emit started();
  return true;
}

void VideoFileSource::stop() {
  if (!m_isActive)
    return;

  LOG_INFO("Stopping VideoFileSource");
  m_isActive = false;
  m_stopDecode = true;
  m_queueCond.notify_all();

  if (m_decodeThread.joinable()) {
    m_decodeThread.join();
  }

  emit stopped();
}

bool VideoFileSource::openFile(const QString &filePath) {
  if (m_formatCtx)
    closeFile();

  m_filePath = filePath;

  if (avformat_open_input(&m_formatCtx, filePath.toUtf8().constData(), nullptr,
                          nullptr) != 0) {
    LOG_ERROR("Failed to open file: " + filePath);
    return false;
  }

  if (avformat_find_stream_info(m_formatCtx, nullptr) < 0) {
    LOG_ERROR("Failed to find stream info");
    return false;
  }

  // Find video stream
  const AVCodec *codec = nullptr;
  m_videoStreamIndex =
      av_find_best_stream(m_formatCtx, AVMEDIA_TYPE_VIDEO, -1, -1, &codec, 0);
  if (m_videoStreamIndex < 0) {
    LOG_ERROR("No video stream found");
    return false;
  }

  m_codecCtx = avcodec_alloc_context3(codec);
  avcodec_parameters_to_context(
      m_codecCtx, m_formatCtx->streams[m_videoStreamIndex]->codecpar);

  if (avcodec_open2(m_codecCtx, codec, nullptr) < 0) {
    LOG_ERROR("Failed to open codec");
    return false;
  }

  // Initialize SwsContext for UYVY conversion (DeckLink format)
  // We create it on the fly when we know frame dimensions

  LOG_INFO("Opened file: " + filePath);
  return true;
}

void VideoFileSource::closeFile() {
  stop();

  if (m_swsCtx) {
    sws_freeContext(m_swsCtx);
    m_swsCtx = nullptr;
  }
  if (m_codecCtx) {
    avcodec_free_context(&m_codecCtx);
  }
  if (m_formatCtx) {
    avformat_close_input(&m_formatCtx);
  }
  m_filePath.clear();
}

void VideoFileSource::decodeLoop() {
  AVPacket *packet = av_packet_alloc();
  AVFrame *frame = av_frame_alloc();
  AVFrame *outputFrame = av_frame_alloc();

  while (!m_stopDecode) {
    if (m_isPaused) {
      std::this_thread::sleep_for(std::chrono::milliseconds(10));
      continue;
    }

    // Queue size check
    {
      QMutexLocker locker(&m_queueMutex);
      if (m_frameQueue.size() >= MAX_QUEUE_SIZE) {
        // Wait for consumption or stop
        // (Simplified sleep for reconstruction)
        locker.unlock();
        std::this_thread::sleep_for(std::chrono::milliseconds(5));
        continue;
      }
    }

    if (av_read_frame(m_formatCtx, packet) >= 0) {
      if (packet->stream_index == m_videoStreamIndex) {
        if (avcodec_send_packet(m_codecCtx, packet) == 0) {
          while (avcodec_receive_frame(m_codecCtx, frame) == 0) {
            // Convert to UYVY
            if (!m_swsCtx) {
              m_swsCtx = sws_getContext(
                  frame->width, frame->height, m_codecCtx->pix_fmt, 1920, 1080,
                  AV_PIX_FMT_UYVY422, SWS_BILINEAR, nullptr, nullptr, nullptr);
            }

            // Prepare output frame buffer
            int numBytes =
                av_image_get_buffer_size(AV_PIX_FMT_UYVY422, 1920, 1080, 1);
            QByteArray buffer(numBytes, 0);

            uint8_t *destData[4] = {(uint8_t *)buffer.data(), nullptr, nullptr,
                                    nullptr};
            int destLinesize[4] = {1920 * 2, 0, 0,
                                   0}; // UYVY is 2 bytes per pixel

            sws_scale(m_swsCtx, frame->data, frame->linesize, 0, frame->height,
                      destData, destLinesize);

            // Push to queue
            VideoFrame vFrame;
            vFrame.data = buffer;
            vFrame.pts = frame->pts; // Simplify PTS
            vFrame.width = 1920;
            vFrame.height = 1080;

            {
              QMutexLocker locker(&m_queueMutex);
              m_frameQueue.push(vFrame);

              // Update preview occasionaly
              if (m_frameQueue.size() % 5 == 0) {
                QMutexLocker pLock(&m_previewMutex);
                // Very basic UYVY to RGB for preview
                // In real app we might use SwsContext again or shader
                // For now, simple box or just logic
              }
            }
          }
        }
      }
      av_packet_unref(packet);
    } else {
      // EOF, loop
      av_seek_frame(m_formatCtx, m_videoStreamIndex, 0, AVSEEK_FLAG_BACKWARD);
    }
  }

  av_frame_free(&outputFrame);
  av_frame_free(&frame);
  av_packet_free(&packet);
}

bool VideoFileSource::fillNextFrame(IDeckLinkMutableVideoFrame *frame) {
  if (!m_isActive)
    return false;

  VideoFrame vFrame;
  {
    QMutexLocker locker(&m_queueMutex);
    if (m_frameQueue.empty()) {
      return false; // Underrun
    }
    vFrame = m_frameQueue.front();
    m_frameQueue.pop();
  }

  // Write to DeckLink Frame using StartAccess fix
  IDeckLinkVideoBuffer *videoBuffer = nullptr;
  if (frame->QueryInterface(IID_IDeckLinkVideoBuffer, (void **)&videoBuffer) ==
      S_OK) {
    void *buffer = nullptr;
    if (videoBuffer->StartAccess(bmdBufferAccessWrite) == S_OK) {
      if (videoBuffer->GetBytes(&buffer) == S_OK) {
        // Copy data
        long rowBytes = frame->GetRowBytes();
        long height = frame->GetHeight();
        // Ensure we don't overflow
        long copySize = std::min((long)vFrame.data.size(), rowBytes * height);
        memcpy(buffer, vFrame.data.constData(), copySize);
      }
      videoBuffer->EndAccess(bmdBufferAccessWrite);
    }
    videoBuffer->Release();
    return true;
  }

  return false;
}

void VideoFileSource::fillQImage(QImage &image) {
  // Basic preview fallback (green box to show it's active)
  QPainter p(&image);
  p.fillRect(image.rect(), Qt::black);
  p.setPen(Qt::white);
  p.drawText(image.rect(), Qt::AlignCenter,
             "Video Playing\n(Preview optimize pending)");

  // In full implementation we would convert UYVY to RGB
  // But for reconstruction speed, placeholder is fine
}

void VideoFileSource::play() { m_isPaused = false; }
void VideoFileSource::pause() { m_isPaused = true; }
void VideoFileSource::seek(int64_t timestamp) {
  // Implement seek
}
int64_t VideoFileSource::getDuration() const {
  if (m_formatCtx)
    return m_formatCtx->duration;
  return 0;
}
int64_t VideoFileSource::getPosition() const { return 0; }

} // namespace Monitor3G
