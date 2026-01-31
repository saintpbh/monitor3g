#ifndef LOGGER_H
#define LOGGER_H

#include <QDateTime>
#include <QFile>
#include <QMutex>
#include <QString>
#include <QTextStream>
#include <functional>
#include <memory>
#include <vector>

namespace Monitor3G {

enum class LogLevel { DEBUG, INFO, WARNING, ERROR, CRITICAL };

using LogCallback = std::function<void(LogLevel, const QString &)>;

class Logger {
public:
  static Logger &instance();

  void setLogLevel(LogLevel level);
  void setLogFile(const QString &filename);
  void addCallback(LogCallback callback);

  void debug(const QString &message);
  void info(const QString &message);
  void warning(const QString &message);
  void error(const QString &message);
  void critical(const QString &message);

  void log(LogLevel level, const QString &message);

private:
  Logger();
  ~Logger();
  Logger(const Logger &) = delete;
  Logger &operator=(const Logger &) = delete;

  void writeLog(LogLevel level, const QString &message);
  QString levelToString(LogLevel level) const;

  LogLevel m_logLevel;
  std::unique_ptr<QFile> m_logFile;
  std::unique_ptr<QTextStream> m_logStream;
  QMutex m_mutex;
  std::vector<LogCallback> m_callbacks;
};

// Convenience macros
#define LOG_DEBUG(msg) Monitor3G::Logger::instance().debug(msg)
#define LOG_INFO(msg) Monitor3G::Logger::instance().info(msg)
#define LOG_WARNING(msg) Monitor3G::Logger::instance().warning(msg)
#define LOG_ERROR(msg) Monitor3G::Logger::instance().error(msg)
#define LOG_CRITICAL(msg) Monitor3G::Logger::instance().critical(msg)

} // namespace Monitor3G

#endif // LOGGER_H
