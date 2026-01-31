#ifndef LOGMESSENGER_H
#define LOGMESSENGER_H

#include "Logger.h"
#include <QObject>
#include <QString>

namespace Monitor3G {

class LogMessenger : public QObject {
  Q_OBJECT
public:
  explicit LogMessenger(QObject *parent = nullptr) : QObject(parent) {
    Logger::instance().addCallback(
        [this](LogLevel level, const QString &msg) { emit logReceived(msg); });
  }

signals:
  void logReceived(const QString &message);
};

} // namespace Monitor3G

#endif // LOGMESSENGER_H
