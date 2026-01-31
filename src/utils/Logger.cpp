#include "Logger.h"
#include <iostream>

namespace Monitor3G {

Logger &Logger::instance() {
  static Logger instance;
  return instance;
}

Logger::Logger() : m_logLevel(LogLevel::INFO) {}

Logger::~Logger() {
  if (m_logFile && m_logFile->isOpen()) {
    m_logFile->close();
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

void Logger::addCallback(LogCallback callback) {
  QMutexLocker locker(&m_mutex);
  m_callbacks.push_back(callback);
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
  {
    QMutexLocker locker(&m_mutex);
    if (level < m_logLevel)
      return;
  }
  writeLog(level, message);
}

void Logger::writeLog(LogLevel level, const QString &message) {
  QMutexLocker locker(&m_mutex);

  QString timestamp =
      QDateTime::currentDateTime().toString("yyyy-MM-dd HH:mm:ss.zzz");
  QString levelStr = levelToString(level);
  QString formattedMessage =
      QString("[%1] [%2] %3").arg(timestamp, levelStr, message);

  // Console output
  std::cout << formattedMessage.toStdString() << std::endl;

  // File output
  if (m_logStream) {
    *m_logStream << formattedMessage << "\n";
    m_logStream->flush();
  }

  // Callbacks
  for (const auto &callback : m_callbacks) {
    callback(level, formattedMessage);
  }
}

QString Logger::levelToString(LogLevel level) const {
  switch (level) {
  case LogLevel::DEBUG:
    return "DEBUG";
  case LogLevel::INFO:
    return "INFO "; // Extra space for alignment
  case LogLevel::WARNING:
    return "WARN ";
  case LogLevel::ERROR:
    return "ERROR";
  case LogLevel::CRITICAL:
    return "FATAL";
  default:
    return "UNKNOWN";
  }
}

} // namespace Monitor3G
