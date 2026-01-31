#include "AudioManager.h"
#include "../utils/Logger.h"

namespace Monitor3G {

AudioManager::AudioManager(QObject *parent)
    : QObject(parent), m_devices(new QMediaDevices(this)) {
  m_currentDevice = QMediaDevices::defaultAudioOutput();
  connect(m_devices, &QMediaDevices::audioOutputsChanged, this,
          &AudioManager::onAvailableDevicesChanged);
}

AudioManager::~AudioManager() {}

QStringList AudioManager::getAvailableOutputDevices() const {
  QStringList names;
  const auto devices = QMediaDevices::audioOutputs();
  for (const auto &device : devices) {
    names << device.description();
  }
  return names;
}

void AudioManager::setOutputDevice(int index) {
  const auto devices = QMediaDevices::audioOutputs();
  if (index >= 0 && index < devices.size()) {
    m_currentDevice = devices[index];
    emit currentDeviceChanged(m_currentDevice);
    LOG_INFO(
        QString("Audio output set to: %1").arg(m_currentDevice.description()));
  }
}

QAudioDevice AudioManager::currentDevice() const { return m_currentDevice; }

void AudioManager::onAvailableDevicesChanged() {
  emit devicesChanged();
  LOG_INFO("Audio output devices updated");
}

} // namespace Monitor3G
