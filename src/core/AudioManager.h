#ifndef AUDIOMANAGER_H
#define AUDIOMANAGER_H

#include <QAudioDevice>
#include <QMediaDevices>
#include <QObject>
#include <QStringList>

namespace Monitor3G {

class AudioManager : public QObject {
  Q_OBJECT
public:
  explicit AudioManager(QObject *parent = nullptr);
  virtual ~AudioManager();

  QStringList getAvailableOutputDevices() const;
  void setOutputDevice(int index);
  QAudioDevice currentDevice() const;

signals:
  void devicesChanged();
  void currentDeviceChanged(const QAudioDevice &device);

private slots:
  void onAvailableDevicesChanged();

private:
  QAudioDevice m_currentDevice;
  QMediaDevices *m_devices;
};

} // namespace Monitor3G

#endif // AUDIOMANAGER_H
