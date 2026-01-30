#include "DeckLinkDevice.h"
#include "../utils/ErrorHandler.h"
#include "../utils/Logger.h"

#ifdef __APPLE__
#include <CoreFoundation/CoreFoundation.h>
#endif

namespace Monitor3G {

DeckLinkDevice::DeckLinkDevice() : m_deckLink(nullptr), m_isOpen(false) {}

DeckLinkDevice::~DeckLinkDevice() { closeDevice(); }

QList<DeviceInfo> DeckLinkDevice::enumerateDevices() {
  QList<DeviceInfo> devices;

  IDeckLinkIterator *deckLinkIterator = CreateDeckLinkIteratorInstance();
  if (!deckLinkIterator) {
    LOG_ERROR("Failed to create DeckLink iterator. Is the DeckLink driver "
              "installed?");
    return devices;
  }

  IDeckLink *deckLink = nullptr;
  while (deckLinkIterator->Next(&deckLink) == S_OK) {
    DeviceInfo info;

    // Get display name
#ifdef __APPLE__
    CFStringRef displayNameCF = nullptr;
    if (deckLink->GetDisplayName(&displayNameCF) == S_OK) {
      char displayName[256];
      CFStringGetCString(displayNameCF, displayName, sizeof(displayName),
                         kCFStringEncodingUTF8);
      info.displayName = QString::fromUtf8(displayName);
      CFRelease(displayNameCF);
    }
#elif _WIN32
    BSTR displayNameBSTR = nullptr;
    if (deckLink->GetDisplayName(&displayNameBSTR) == S_OK) {
      info.displayName = QString::fromWCharArray(displayNameBSTR);
      SysFreeString(displayNameBSTR);
    }
#endif

    // Get model name and capabilities
    IDeckLinkProfileAttributes *attributes = nullptr;
    if (deckLink->QueryInterface(IID_IDeckLinkProfileAttributes,
                                 (void **)&attributes) == S_OK) {
      int64_t deviceID = 0;
      if (attributes->GetInt(BMDDeckLinkPersistentID, &deviceID) == S_OK) {
        info.persistentID = deviceID;
      }

      // TODO: Query input/output support
      info.supportsInput = true;  // Placeholder
      info.supportsOutput = true; // Placeholder

      attributes->Release();
    }

    info.modelName = info.displayName; // For now, use same as display name
    devices.append(info);

    deckLink->Release();
  }

  deckLinkIterator->Release();

  LOG_INFO(QString("Found %1 DeckLink device(s)").arg(devices.size()));
  return devices;
}

bool DeckLinkDevice::openDevice(int deviceIndex) {
  if (m_isOpen) {
    LOG_WARNING("Device already open. Closing previous device.");
    closeDevice();
  }

  IDeckLinkIterator *deckLinkIterator = CreateDeckLinkIteratorInstance();
  if (!deckLinkIterator) {
    ErrorHandler::instance().handleError(ErrorCode::DEVICE_NOT_FOUND,
                                         "Failed to create DeckLink iterator");
    return false;
  }

  IDeckLink *deckLink = nullptr;
  int currentIndex = 0;

  while (deckLinkIterator->Next(&deckLink) == S_OK) {
    if (currentIndex == deviceIndex) {
      m_deckLink = deckLink;
      deckLinkIterator->Release();

      if (queryDeviceInfo()) {
        m_isOpen = true;
        LOG_INFO(QString("Opened DeckLink device: %1")
                     .arg(m_deviceInfo.displayName));
        return true;
      } else {
        m_deckLink->Release();
        m_deckLink = nullptr;
        ErrorHandler::instance().handleError(
            ErrorCode::DEVICE_INIT_FAILED,
            "Failed to query device information");
        return false;
      }
    }

    deckLink->Release();
    currentIndex++;
  }

  deckLinkIterator->Release();

  ErrorHandler::instance().handleError(
      ErrorCode::DEVICE_NOT_FOUND,
      QString("Device index %1 not found").arg(deviceIndex));
  return false;
}

bool DeckLinkDevice::openDeviceByID(int64_t persistentID) {
  QList<DeviceInfo> devices = enumerateDevices();

  for (int i = 0; i < devices.size(); ++i) {
    if (devices[i].persistentID == persistentID) {
      return openDevice(i);
    }
  }

  ErrorHandler::instance().handleError(
      ErrorCode::DEVICE_NOT_FOUND,
      QString("Device with ID %1 not found").arg(persistentID));
  return false;
}

void DeckLinkDevice::closeDevice() {
  if (m_deckLink) {
    m_deckLink->Release();
    m_deckLink = nullptr;
  }
  m_isOpen = false;
  LOG_INFO("DeckLink device closed");
}

bool DeckLinkDevice::isOpen() const { return m_isOpen; }

DeviceInfo DeckLinkDevice::getDeviceInfo() const { return m_deviceInfo; }

QString DeckLinkDevice::getDeviceName() const {
  return m_deviceInfo.displayName;
}

bool DeckLinkDevice::queryDeviceInfo() {
  if (!m_deckLink) {
    return false;
  }

  // Get display name
#ifdef __APPLE__
  CFStringRef displayNameCF = nullptr;
  if (m_deckLink->GetDisplayName(&displayNameCF) == S_OK) {
    char displayName[256];
    CFStringGetCString(displayNameCF, displayName, sizeof(displayName),
                       kCFStringEncodingUTF8);
    m_deviceInfo.displayName = QString::fromUtf8(displayName);
    CFRelease(displayNameCF);
  }
#elif _WIN32
  BSTR displayNameBSTR = nullptr;
  if (m_deckLink->GetDisplayName(&displayNameBSTR) == S_OK) {
    m_deviceInfo.displayName = QString::fromWCharArray(displayNameBSTR);
    SysFreeString(displayNameBSTR);
  }
#endif

  // Get attributes
  IDeckLinkProfileAttributes *attributes = nullptr;
  if (m_deckLink->QueryInterface(IID_IDeckLinkProfileAttributes,
                                 (void **)&attributes) == S_OK) {
    int64_t deviceID = 0;
    if (attributes->GetInt(BMDDeckLinkPersistentID, &deviceID) == S_OK) {
      m_deviceInfo.persistentID = deviceID;
    }

    attributes->Release();
  }

  m_deviceInfo.modelName = m_deviceInfo.displayName;
  m_deviceInfo.supportsInput = true; // TODO: Query actual capabilities
  m_deviceInfo.supportsOutput = true;

  return true;
}

} // namespace Monitor3G
