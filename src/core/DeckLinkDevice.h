#ifndef DECKLINK_DEVICE_H
#define DECKLINK_DEVICE_H

#include <QList>
#include <QString>
#include <memory>

#ifdef __APPLE__
#include "DeckLinkAPI.h"
#elif _WIN32
#include "DeckLinkAPI.h"
#endif

namespace Monitor3G {

struct DeviceInfo {
  QString displayName;
  QString modelName;
  int64_t persistentID;
  bool supportsInput;
  bool supportsOutput;
};

class DeckLinkDevice {
public:
  DeckLinkDevice();
  ~DeckLinkDevice();

  // Device enumeration
  static QList<DeviceInfo> enumerateDevices();

  // Device management
  bool openDevice(int deviceIndex = 0);
  bool openDeviceByID(int64_t persistentID);
  void closeDevice();
  bool isOpen() const;

  // Device information
  DeviceInfo getDeviceInfo() const;
  QString getDeviceName() const;

  // Get raw DeckLink interface (for internal use)
  IDeckLink *getDeckLinkInterface() const { return m_deckLink; }

private:
  IDeckLink *m_deckLink;
  DeviceInfo m_deviceInfo;
  bool m_isOpen;

  bool queryDeviceInfo();
};

} // namespace Monitor3G

#endif // DECKLINK_DEVICE_H
