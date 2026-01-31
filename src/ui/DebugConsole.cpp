#include "DebugConsole.h"
#include "../utils/Logger.h"
#include <QApplication>
#include <QClipboard>
#include <QHBoxLayout>
#include <QLabel>
#include <QPlainTextEdit>
#include <QPushButton>
#include <QVBoxLayout>

namespace Monitor3G {

DebugConsole::DebugConsole(QWidget *parent) : QWidget(parent) {
  setupUI();

  // Connect to Logger
  connect(Logger::instance().messenger(), &LogMessenger::logReceived, this,
          &DebugConsole::appendLog);
}

void DebugConsole::setupUI() {
  QVBoxLayout *layout = new QVBoxLayout(this);
  layout->setContentsMargins(0, 0, 0, 0);
  layout->setSpacing(0);

  // Toolbar
  QWidget *toolbar = new QWidget(this);
  toolbar->setStyleSheet(
      "background-color: #2b2b2b; border-bottom: 1px solid #3d3d3d;");
  QHBoxLayout *hLayout = new QHBoxLayout(toolbar);
  hLayout->setContentsMargins(10, 5, 10, 5);

  QLabel *title = new QLabel(tr("DEVELOPER CONSOLE"), this);
  title->setStyleSheet("color: #aaaaaa; font-weight: bold; font-size: 10px;");
  hLayout->addWidget(title);
  hLayout->addStretch();

  m_copyBtn = new QPushButton(tr("Copy All"), this);
  m_copyBtn->setStyleSheet("background-color: #3d3d3d; color: white; border: "
                           "none; padding: 4px 8px; border-radius: 4px;");
  connect(m_copyBtn, &QPushButton::clicked, this, &DebugConsole::copyAll);
  hLayout->addWidget(m_copyBtn);

  m_clearBtn = new QPushButton(tr("Clear"), this);
  m_clearBtn->setStyleSheet("background-color: #3d3d3d; color: white; border: "
                            "none; padding: 4px 8px; border-radius: 4px;");
  connect(m_clearBtn, &QPushButton::clicked, this, &DebugConsole::clear);
  hLayout->addWidget(m_clearBtn);

  layout->addWidget(toolbar);

  // Text Area
  m_textEdit = new QPlainTextEdit(this);
  m_textEdit->setReadOnly(true);
  m_textEdit->setStyleSheet(
      "background-color: #1e1e1e; "
      "color: #d4d4d4; "
      "font-family: 'Monaco', 'Menlo', 'Courier New', monospace; "
      "font-size: 12px; "
      "border: none;");
  layout->addWidget(m_textEdit);

  // Initial message
  appendLog("--- Console Ready ---", LogLevel::INFO);
}

void DebugConsole::appendLog(const QString &message,
                             Monitor3G::LogLevel level) {
  QString color;
  switch (level) {
  case LogLevel::DEBUG:
    color = "#808080";
    break;
  case LogLevel::INFO:
    color = "#d4d4d4";
    break;
  case LogLevel::WARNING:
    color = "#ce9178";
    break;
  case LogLevel::ERROR:
    color = "#f44747";
    break;
  case LogLevel::CRITICAL:
    color = "#ff0000";
    break;
  default:
    color = "#d4d4d4";
    break;
  }

  // Since we use appendPlainText, we can't do per-line HTML colors easily
  // without using insertHtml. Let's use simple text for now or rich text if
  // needed. We'll stick to simple text to avoid overhead.
  m_textEdit->appendPlainText(message);

  // Auto-scroll to bottom
  m_textEdit->moveCursor(QTextCursor::End);
}

void DebugConsole::clear() { m_textEdit->clear(); }

void DebugConsole::copyAll() {
  QApplication::clipboard()->setText(m_textEdit->toPlainText());
}

} // namespace Monitor3G
