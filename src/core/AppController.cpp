#include "AppController.h"
#include "../sources/TestPatternSource.h"
#include "../sources/VideoFileSource.h"
#include "../ui/qml/FrameProvider.h"
#include "../utils/Logger.h"
#include "DeckLinkDevice.h"
#include "DeckLinkOutput.h"

#include <QFileDialog>
#include <QStandardPaths>

namespace Monitor3G {

AppController::AppController(QObject *parent)
    : QObject(parent), m_output(nullptr), m_testPatternSource(nullptr),
      m_currentMainSource(nullptr), m_previewSource(nullptr),
      m_programSource(nullptr), m_isDeviceConnected(false) {

  m_timer = new QTimer(this);
  connect(m_timer, &QTimer::timeout, this, &AppController::updatePreview);
  m_timer->setInterval(33); // ~30 FPS
}

AppController::~AppController() {
  if (m_output) {
    m_output->stop();
    delete m_output;
  }
  if (m_device) {
    m_device->closeDevice();
  }
  // Cleanup sources...
}

void AppController::initialize() {
  m_device = std::make_unique<DeckLinkDevice>();
  m_output = new DeckLinkOutput(this);
  m_testPatternSource = new TestPatternSource(this);
  m_testPatternSource->setPattern(TestPattern::ColorBars);

  // Default sources
  m_sources.append(m_testPatternSource);
  m_sourceNames.append("Test Pattern (Color Bars)");
  emit sourceListChanged();

  // Try to open device
  if (m_device->openDevice(0)) {
    LOG_INFO("Opened DeckLink device: " + m_device->getDeviceName());
    if (m_output->initialize(m_device->getDeckLinkInterface())) {
      m_deviceStatusMsg = "DEVICE: " + m_device->getDeviceName();
      m_isDeviceConnected = true;
    } else {
      m_deviceStatusMsg = "DEVICE: ERROR";
      m_isDeviceConnected = false;
    }
  } else {
    m_deviceStatusMsg = "DEVICE: SIMULATION MODE";
    m_isDeviceConnected = false;
  }
  emit deviceStatusChanged();

  // Select default source
  selectSource(0);
  m_timer->start();
}

void AppController::updatePreview() {
  // Update PREVIEW monitor
  if (m_previewSource) {
    QImage previewFrame;
    m_previewSource->fillQImage(previewFrame);
    if (!previewFrame.isNull()) {
      FrameProvider::instance().deliverPreviewFrame(previewFrame);
    }
  }

  // Update PROGRAM monitor
  if (m_programSource) {
    QImage programFrame;
    m_programSource->fillQImage(programFrame);
    if (!programFrame.isNull()) {
      FrameProvider::instance().deliverProgramFrame(programFrame);
    }
  }

  // Update playback position if video (use current preview for transport
  // controls)
  if (auto vs = qobject_cast<VideoFileSource *>(m_previewSource)) {
    emit positionChanged();
    // Auto-update playback state if it changed externally (e.g. EOF)
    static bool lastPaused = false;
    if (vs->isPaused() != lastPaused) {
      lastPaused = vs->isPaused();
      emit isPlayingChanged();
    }
  }
}

void AppController::selectSource(int index) {
  if (index >= 0 && index < m_sources.size()) {
    // Select source goes to PREVIEW
    m_previewSource = m_sources[index];
    m_currentMainSource = m_previewSource; // For backward compatibility

    // Auto play if video
    if (auto vs = qobject_cast<VideoFileSource *>(m_previewSource)) {
      vs->start();
      vs->play();
    }

    LOG_INFO("Selected source to Preview: " + QString::number(index));

    emit sourceTypeChanged();
    emit durationChanged();
    emit isPlayingChanged();
    emit positionChanged();
  }
}

void AppController::requestAddFile() {
  QString fileName = QFileDialog::getOpenFileName(
      nullptr, tr("Open Video"),
      QStandardPaths::writableLocation(QStandardPaths::MoviesLocation),
      tr("Video Files (*.mp4 *.mov *.avi *.mkv)"));

  if (!fileName.isEmpty()) {
    VideoFileSource *source = new VideoFileSource(this);
    if (source->openFile(fileName)) {
      m_sources.append(source);
      m_sourceNames.append(QFileInfo(fileName).fileName());
      emit sourceListChanged();
    } else {
      LOG_ERROR("Failed to open video: " + fileName);
      source->deleteLater();
    }
  }
}

void AppController::startOutput() {
  if (m_output) {
    m_output->start();
    if (m_currentMainSource)
      m_currentMainSource->start();
  }
}

void AppController::stopOutput() {
  if (m_output) {
    m_output->stop();
  }
}

QString AppController::deviceStatus() const { return m_deviceStatusMsg; }
bool AppController::isDeviceConnected() const { return m_isDeviceConnected; }
QStringList AppController::sourceList() const { return m_sourceNames; }

bool AppController::isPlaying() const {
  if (auto vs = qobject_cast<VideoFileSource *>(m_currentMainSource)) {
    return !vs->isPaused();
  }
  return false;
}

qint64 AppController::position() const {
  if (auto vs = qobject_cast<VideoFileSource *>(m_currentMainSource)) {
    return vs->getPosition();
  }
  return 0;
}

qint64 AppController::duration() const {
  if (auto vs = qobject_cast<VideoFileSource *>(m_currentMainSource)) {
    return vs->getDuration();
  }
  return 0;
}

int AppController::sourceType() const {
  if (!m_currentMainSource)
    return 0;
  // Simple mapping: 1=Video, 0=Others for now
  if (qobject_cast<VideoFileSource *>(m_currentMainSource))
    return 1;
  return 0;
}

void AppController::play() {
  if (auto vs = qobject_cast<VideoFileSource *>(m_currentMainSource)) {
    vs->play();
    emit isPlayingChanged();
  }
}

void AppController::pause() {
  if (auto vs = qobject_cast<VideoFileSource *>(m_currentMainSource)) {
    vs->pause();
    emit isPlayingChanged();
  }
}

void AppController::seek(qint64 position) {
  if (auto vs = qobject_cast<VideoFileSource *>(m_currentMainSource)) {
    vs->seek(position);
    emit positionChanged();
  }
}

void AppController::takePreview() {
  if (m_previewSource) {
    m_programSource = m_previewSource;
    m_output->setSource(m_programSource);
    LOG_INFO("TAKE: Preview -> Program");
  }
}

} // namespace Monitor3G
