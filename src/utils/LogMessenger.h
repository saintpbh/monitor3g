#ifndef LOGMESSENGER_H
#define LOGMESSENGER_H

#include <QObject>
#include <QString>

namespace Monitor3G {

enum class LogLevel { DEBUG, INFO, WARNING, ERROR, CRITICAL };

class LogMessenger : public QObject {
  Q_OBJECT
signals:
  void logReceived(const QString &message, Monitor3G::LogLevel level);
};

} // namespace Monitor3G

#endif // LOGMESSENGER_H
