#include "Logger.h"
#include <QMutexLocker>
#include <iostream>

namespace Monitor3G {

Logger &Logger::instance() {
  static Logger instance;
  return instance;
}

Logger::Logger() : m_logLevel(LogLevel::INFO) {
  // Default log to stdout
}

Logger::~Logger() {
  if (m_logStream) {
    m_logStream->flush();
  }
}

void Logger::setLogLevel(LogLevel level) {
  QMutexLocker locker(&m_mutex);
  m_logLevel = level;
}

void Logger::setLogFile(const QString &filename) {
  QMutexLocker locker(&m_mutex);

  m_logFile = std::make_unique<QFile>(filename);
  if (m_logFile->open(QIODevice::WriteOnly | QIODevice::Append |
                      QIODevice::Text)) {
    m_logStream = std::make_unique<QTextStream>(m_logFile.get());
  }
}

void Logger::debug(const QString &message) { log(LogLevel::DEBUG, message); }

void Logger::info(const QString &message) { log(LogLevel::INFO, message); }

void Logger::warning(const QString &message) {
  log(LogLevel::WARNING, message);
}

void Logger::error(const QString &message) { log(LogLevel::ERROR, message); }

void Logger::critical(const QString &message) {
  log(LogLevel::CRITICAL, message);
}

void Logger::log(LogLevel level, const QString &message) {
  if (level < m_logLevel) {
    return;
  }

  writeLog(level, message);
}

void Logger::writeLog(LogLevel level, const QString &message) {
  QMutexLocker locker(&m_mutex);

  QString timestamp =
      QDateTime::currentDateTime().toString("yyyy-MM-dd hh:mm:ss.zzz");
  QString levelStr = levelToString(level);
  QString logMessage =
      QString("[%1] [%2] %3").arg(timestamp, levelStr, message);

  // Write to file if available
  if (m_logStream) {
    *m_logStream << logMessage << Qt::endl;
    m_logStream->flush();
  }

  // Also write to console
  if (level >= LogLevel::ERROR) {
    std::cerr << logMessage.toStdString() << std::endl;
  } else {
    std::cout << logMessage.toStdString() << std::endl;
  }
}

QString Logger::levelToString(LogLevel level) const {
  switch (level) {
  case LogLevel::DEBUG:
    return "DEBUG";
  case LogLevel::INFO:
    return "INFO ";
  case LogLevel::WARNING:
    return "WARN ";
  case LogLevel::ERROR:
    return "ERROR";
  case LogLevel::CRITICAL:
    return "CRIT ";
  default:
    return "UNKNOWN";
  }
}

} // namespace Monitor3G
