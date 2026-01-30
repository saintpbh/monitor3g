#include "ErrorHandler.h"
#include "Logger.h"

namespace Monitor3G {

ErrorHandler &ErrorHandler::instance() {
  static ErrorHandler instance;
  return instance;
}

ErrorHandler::ErrorHandler() : QObject(nullptr) {}

void ErrorHandler::handleError(ErrorCode code, const QString &message) {
  QString fullMessage = QString("%1: %2").arg(errorCodeToString(code), message);

  if (isFatalError(code)) {
    LOG_CRITICAL(fullMessage);
    emit fatalError(code, message);

    if (m_fatalErrorCallback) {
      m_fatalErrorCallback(code, message);
    }
  } else {
    LOG_ERROR(fullMessage);
    emit errorOccurred(code, message);
  }
}

void ErrorHandler::setFatalErrorCallback(
    std::function<void(ErrorCode, QString)> callback) {
  m_fatalErrorCallback = callback;
}

QString ErrorHandler::errorCodeToString(ErrorCode code) const {
  switch (code) {
  case ErrorCode::DEVICE_NOT_FOUND:
    return "Device Not Found";
  case ErrorCode::DEVICE_DISCONNECTED:
    return "Device Disconnected";
  case ErrorCode::DEVICE_BUSY:
    return "Device Busy";
  case ErrorCode::DEVICE_INIT_FAILED:
    return "Device Initialization Failed";
  case ErrorCode::OUTPUT_FORMAT_NOT_SUPPORTED:
    return "Output Format Not Supported";
  case ErrorCode::OUTPUT_START_FAILED:
    return "Output Start Failed";
  case ErrorCode::OUTPUT_FRAME_DROP:
    return "Frame Drop Detected";
  case ErrorCode::SOURCE_FILE_NOT_FOUND:
    return "Source File Not Found";
  case ErrorCode::SOURCE_DECODE_ERROR:
    return "Source Decode Error";
  case ErrorCode::SOURCE_UNSUPPORTED_FORMAT:
    return "Unsupported Source Format";
  case ErrorCode::OUT_OF_MEMORY:
    return "Out of Memory";
  case ErrorCode::SYSTEM_ERROR:
    return "System Error";
  default:
    return "Unknown Error";
  }
}

bool ErrorHandler::isFatalError(ErrorCode code) const {
  switch (code) {
  case ErrorCode::DEVICE_INIT_FAILED:
  case ErrorCode::OUT_OF_MEMORY:
  case ErrorCode::SYSTEM_ERROR:
    return true;
  default:
    return false;
  }
}

} // namespace Monitor3G
