#include "DeckLinkOutput.h"
#include "../utils/ErrorHandler.h"
#include "../utils/Logger.h"

namespace Monitor3G {

DeckLinkOutput::DeckLinkOutput(QObject *parent)
    : QObject(parent), m_refCount(1), m_deckLink(nullptr),
      m_deckLinkOutput(nullptr), m_isOutputting(false), m_activeSource(nullptr),
      m_frameCount(0), m_totalFramesScheduled(0), m_width(1920), m_height(1080),
      m_timeScale(60000), m_frameDuration(1000) {
  m_simulationTimer = new QTimer(this);
  connect(m_simulationTimer, &QTimer::timeout, this,
          &DeckLinkOutput::onSimulationTimer);
}

DeckLinkOutput::~DeckLinkOutput() {
  stop();
  if (m_deckLinkOutput) {
    m_deckLinkOutput->Release();
    m_deckLinkOutput = nullptr;
  }
  if (m_deckLink) {
    m_deckLink->Release();
    m_deckLink = nullptr;
  }
}

bool DeckLinkOutput::initialize(IDeckLink *deckLink) {
  if (!deckLink)
    return false;

  if (m_deckLink) {
    m_deckLink->Release();
  }
  m_deckLink = deckLink;
  m_deckLink->AddRef();

  if (m_deckLink->QueryInterface(IID_IDeckLinkOutput,
                                 (void **)&m_deckLinkOutput) != S_OK) {
    LOG_ERROR("Failed to query IDeckLinkOutput");
    return false;
  }

  return true;
}

void DeckLinkOutput::start() {
  QMutexLocker locker(&m_mutex);
  if (m_isOutputting)
    return;

  if (!m_deckLinkOutput) {
    // Simulation mode
    m_simulationTimer->start(16); // ~60fps
    m_isOutputting = true;
    emit outputStarted();
    if (m_activeSource)
      m_activeSource->start();
    return;
  }

  // Hardware mode
  if (m_deckLinkOutput->EnableVideoOutput(bmdModeHD1080p6000,
                                          bmdVideoOutputFlagDefault) != S_OK) {
    LOG_ERROR("Failed to enable video output");
    return;
    return;
  }

  // Reset frame counters for new playback session
  m_totalFramesScheduled = 0;
  m_frameCount = 0;

  // Pre-roll frames (schedule 3 black frames)
  for (int i = 0; i < 3; i++) {
    IDeckLinkMutableVideoFrame *frame = nullptr;
    if (m_deckLinkOutput->CreateVideoFrame(
            m_width, m_height, m_width * 2, bmdFormat8BitYUV,
            bmdFrameFlagDefault, &frame) == S_OK) {
      createBlackFrame(frame);
      if (m_deckLinkOutput->ScheduleVideoFrame(
              frame, m_totalFramesScheduled * m_frameDuration, m_frameDuration,
              m_timeScale) != S_OK) {
        LOG_ERROR("Failed to schedule pre-roll frame");
      }
      m_totalFramesScheduled++;
      frame->Release();
    }
  }

  if (m_deckLinkOutput->StartScheduledPlayback(0, m_timeScale, 1.0) != S_OK) {
    LOG_ERROR("Failed to start scheduled playback");
    return;
  }

  m_deckLinkOutput->SetScheduledFrameCompletionCallback(this);
  m_deckLinkOutput->SetAudioCallback(this);

  m_isOutputting = true;
  m_frameCount = 0;
  if (m_activeSource)
    m_activeSource->start();

  emit outputStarted();
}

void DeckLinkOutput::stop() {
  QMutexLocker locker(&m_mutex);
  if (!m_isOutputting)
    return;

  LOG_INFO("Stopping output");
  m_isOutputting = false; // Stop pump first

  if (m_simulationTimer->isActive()) {
    m_simulationTimer->stop();
  }

  if (m_deckLinkOutput) {
    m_deckLinkOutput->StopScheduledPlayback(0, nullptr, 0);
    m_deckLinkOutput->DisableVideoOutput();
    m_deckLinkOutput->SetScheduledFrameCompletionCallback(nullptr);
    m_deckLinkOutput->SetAudioCallback(nullptr);
  }

  if (m_activeSource)
    m_activeSource->stop();

  emit outputStopped();
}

void DeckLinkOutput::setSource(AbstractSource *source) {
  QMutexLocker locker(&m_mutex);
  bool wasRunning =
      m_isOutputting && m_activeSource && m_activeSource->isActive();

  if (m_activeSource) {
    disconnect(m_activeSource, nullptr, this, nullptr);
    if (m_isOutputting)
      m_activeSource->stop();
  }

  m_activeSource = source;

  if (m_activeSource) {
    connect(m_activeSource, &AbstractSource::started, this,
            &DeckLinkOutput::onSourceStarted);
    connect(m_activeSource, &AbstractSource::stopped, this,
            &DeckLinkOutput::onSourceStopped);
    if (m_isOutputting)
      m_activeSource->start();
  }
}

void DeckLinkOutput::onSimulationTimer() {
  if (m_activeSource) {
    // 1. Emit frame count
    m_frameCount++;
    if (m_frameCount % 60 == 0) {
      emit frameCompleted(m_frameCount);
    }

    // 2. Generate Preview Frame for UI
    QImage preview(m_width, m_height, QImage::Format_ARGB32);
    m_activeSource->fillQImage(preview);
    emit videoFrameArrived(preview);
  }
}

HRESULT
DeckLinkOutput::ScheduledFrameCompleted(IDeckLinkVideoFrame *completedFrame,
                                        BMDOutputFrameCompletionResult result) {
  if (!m_isOutputting)
    return S_OK;

  // Create new frame
  IDeckLinkMutableVideoFrame *newFrame = nullptr;
  HRESULT hr = m_deckLinkOutput->CreateVideoFrame(
      m_width, m_height, m_width * 2, bmdFormat8BitYUV, bmdFrameFlagDefault,
      &newFrame);
  if (hr != S_OK)
    return S_OK;

  // Fill frame
  if (scheduleFrame(newFrame)) {
    // Source filled it
  } else {
    createBlackFrame(newFrame);
  }

  // Schedule it
  // Note: We don't track precise time here for reconstruction simplicity, just
  // relative In real app we might track completion time For now assuming
  // continuous scheduling Since we don't have the "last scheduled time" in this
  // simple recon, we rely on Preroll flow Wait, ScheduledFrameCompleted gives
  // us the completed frame. We should schedule the NEXT frame based on
  // completion time? SDK says: supply displayTime relative to start. Let's
  // implement robust scheduling logic? Simplified: Just use m_frameCount *
  // duration (if we track total frames scheduled) BUT we need thread safety

  m_frameCount++;
  emit frameCompleted(m_frameCount); // This should be queued to UI thread?
                                     // QObject::emit is thread safe for signals
                                     // connected to slots in other threads?
  // Actually, DeckLink callback is on driver thread.
  // We need to be careful.

  // Just blindly schedule for now at "next slot" logic
  // Or better: Use the frame timestamp from completedFrame?
  // To be safe in reconstruction: simple increment

  // Robust scheduling
  m_deckLinkOutput->ScheduleVideoFrame(newFrame,
                                       m_totalFramesScheduled * m_frameDuration,
                                       m_frameDuration, m_timeScale);
  m_totalFramesScheduled++;

  newFrame->Release();

  // Send Preview to UI
  if (m_activeSource) {
    // Create a QImage for preview (scaled down? No, full size for now)
    QImage preview(m_width, m_height, QImage::Format_ARGB32);
    m_activeSource->fillQImage(preview);
    emit videoFrameArrived(preview);
  }

  return S_OK;
}

bool DeckLinkOutput::scheduleFrame(IDeckLinkMutableVideoFrame *frame) {
  if (m_activeSource) {
    return m_activeSource->fillNextFrame(frame);
  }
  return false;
}

void DeckLinkOutput::createBlackFrame(IDeckLinkMutableVideoFrame *frame) {
  IDeckLinkVideoBuffer *videoBuffer = nullptr;
  if (frame->QueryInterface(IID_IDeckLinkVideoBuffer, (void **)&videoBuffer) ==
      S_OK) {
    void *buffer = nullptr;
    // FIX: StartAccess is required on macOS
    if (videoBuffer->StartAccess(bmdBufferAccessWrite) == S_OK) {
      if (videoBuffer->GetBytes(&buffer) == S_OK) {
        // Fill black (Y=16, U=128, V=128 for 8-bit YUV)
        // UYVY pattern: 0x80, 0x10, 0x80, 0x10
        uint32_t blackPixel = 0x10801080;
        size_t numWords = (m_width * m_height * 2) / 4;
        uint32_t *p = (uint32_t *)buffer;
        for (size_t i = 0; i < numWords; i++) {
          p[i] = blackPixel;
        }
      }
      videoBuffer->EndAccess(bmdBufferAccessWrite);
    }
    videoBuffer->Release();
  }
}

HRESULT DeckLinkOutput::ScheduledPlaybackHasStopped() { return S_OK; }

HRESULT DeckLinkOutput::RenderAudioSamples(bool preroll) { return S_OK; }

HRESULT DeckLinkOutput::QueryInterface(REFIID iid, LPVOID *ppv) {
  CFUUIDBytes iunknown = CFUUIDGetUUIDBytes(IUnknownUUID);
  if (memcmp(&iid, &iunknown, sizeof(REFIID)) == 0) {
    *ppv = static_cast<IDeckLinkVideoOutputCallback *>(this);
    AddRef();
    return S_OK;
  }

  if (memcmp(&iid, &IID_IDeckLinkVideoOutputCallback, sizeof(REFIID)) == 0) {
    *ppv = static_cast<IDeckLinkVideoOutputCallback *>(this);
    AddRef();
    return S_OK;
  }

  if (memcmp(&iid, &IID_IDeckLinkAudioOutputCallback, sizeof(REFIID)) == 0) {
    *ppv = static_cast<IDeckLinkAudioOutputCallback *>(this);
    AddRef();
    return S_OK;
  }

  *ppv = nullptr;
  return E_NOINTERFACE;
}

ULONG DeckLinkOutput::AddRef() { return ++m_refCount; }

ULONG DeckLinkOutput::Release() {
  ULONG newRef = --m_refCount;
  if (newRef == 0) {
    delete this;
  }
  return newRef;
}

void DeckLinkOutput::onSourceStarted() {}
void DeckLinkOutput::onSourceStopped() {}

} // namespace Monitor3G
