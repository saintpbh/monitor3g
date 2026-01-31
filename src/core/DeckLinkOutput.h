#ifndef DECKLINK_OUTPUT_H
#define DECKLINK_OUTPUT_H

#include "../sources/AbstractSource.h"
#include "DeckLinkAPI.h"
#include <QMutex>
#include <QObject>
#include <QTimer>
#include <atomic>

namespace Monitor3G {

class DeckLinkOutput : public QObject,
                       public IDeckLinkVideoOutputCallback,
                       public IDeckLinkAudioOutputCallback {
  Q_OBJECT

public:
  explicit DeckLinkOutput(QObject *parent = nullptr);
  ~DeckLinkOutput() override;

  // DeckLink Interface
  bool initialize(IDeckLink *deckLink);
  void start();
  void stop();
  bool isOutputting() const { return m_isOutputting; }

  // Source Management
  void setSource(AbstractSource *source);
  AbstractSource *getSource() const { return m_activeSource; }

  // IUnknown
  HRESULT STDMETHODCALLTYPE QueryInterface(REFIID iid, LPVOID *ppv) override;
  ULONG STDMETHODCALLTYPE AddRef() override;
  ULONG STDMETHODCALLTYPE Release() override;

  // IDeckLinkVideoOutputCallback
  HRESULT STDMETHODCALLTYPE
  ScheduledFrameCompleted(IDeckLinkVideoFrame *completedFrame,
                          BMDOutputFrameCompletionResult result) override;
  HRESULT STDMETHODCALLTYPE ScheduledPlaybackHasStopped() override;

  // IDeckLinkAudioOutputCallback
  HRESULT STDMETHODCALLTYPE RenderAudioSamples(bool preroll) override;

signals:
  void errorOccurred(const QString &message);
  void outputStarted();
  void outputStopped();
  void frameCompleted(int count);
  void videoFrameArrived(const QImage &image);

private slots:
  void onSimulationTimer();
  void onSourceStarted();
  void onSourceStopped();

private:
  bool scheduleFrame(IDeckLinkMutableVideoFrame *frame);
  void createBlackFrame(IDeckLinkMutableVideoFrame *frame);

  // Core members
  std::atomic<ULONG> m_refCount;
  IDeckLink *m_deckLink;
  IDeckLinkOutput *m_deckLinkOutput;

  // State
  std::atomic<bool> m_isOutputting;
  QMutex m_mutex;

  // Source
  AbstractSource *m_activeSource;
  int m_frameCount;           // Frames completed
  int m_totalFramesScheduled; // Frames pushed to card

  // Simulation
  QTimer *m_simulationTimer;

  // Config
  long m_width;
  long m_height;
  BMDTimeScale m_timeScale;
  BMDTimeValue m_frameDuration;
};

} // namespace Monitor3G

#endif // DECKLINK_OUTPUT_H
