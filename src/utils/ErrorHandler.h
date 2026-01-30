#ifndef ERROR_HANDLER_H
#define ERROR_HANDLER_H

#include <QObject>
#include <QString>
#include <functional>

namespace Monitor3G {

enum class ErrorCode {
  // Device errors
  DEVICE_NOT_FOUND,
  DEVICE_DISCONNECTED,
  DEVICE_BUSY,
  DEVICE_INIT_FAILED,

  // Output errors
  OUTPUT_FORMAT_NOT_SUPPORTED,
  OUTPUT_START_FAILED,
  OUTPUT_FRAME_DROP,

  // Source errors
  SOURCE_FILE_NOT_FOUND,
  SOURCE_DECODE_ERROR,
  SOURCE_UNSUPPORTED_FORMAT,

  // System errors
  OUT_OF_MEMORY,
  SYSTEM_ERROR,

  UNKNOWN_ERROR
};

class ErrorHandler : public QObject {
  Q_OBJECT

public:
  static ErrorHandler &instance();

  void handleError(ErrorCode code, const QString &message);
  void setFatalErrorCallback(std::function<void(ErrorCode, QString)> callback);

  QString errorCodeToString(ErrorCode code) const;
  bool isFatalError(ErrorCode code) const;

signals:
  void errorOccurred(ErrorCode code, QString message);
  void fatalError(ErrorCode code, QString message);

private:
  ErrorHandler();
  ~ErrorHandler() = default;
  ErrorHandler(const ErrorHandler &) = delete;
  ErrorHandler &operator=(const ErrorHandler &) = delete;

  std::function<void(ErrorCode, QString)> m_fatalErrorCallback;
};

} // namespace Monitor3G

#endif // ERROR_HANDLER_H
