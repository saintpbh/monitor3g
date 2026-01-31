#include "DeveloperConsole.h"
#include <QApplication>
#include <QClipboard>
#include <QScrollBar>

namespace Monitor3G {

DeveloperConsole::DeveloperConsole(QWidget *parent) : QWidget(parent) {
  setupUI();
  m_messenger = new LogMessenger(this);
  connect(m_messenger, &LogMessenger::logReceived, this,
          &DeveloperConsole::appendLog);
}

void DeveloperConsole::setupUI() {
  QVBoxLayout *mainLayout = new QVBoxLayout(this);
  mainLayout->setContentsMargins(4, 4, 4, 4);
  mainLayout->setSpacing(2);

  // Header
  QHBoxLayout *headerLayout = new QHBoxLayout();
  headerLayout->setContentsMargins(0, 0, 0, 0);

  QLabel *title = new QLabel("DEVELOPER CONSOLE", this);
  title->setStyleSheet("font-weight: bold; font-size: 10px; color: #888;");
  headerLayout->addWidget(title);
  headerLayout->addStretch();

  m_copyBtn = new QPushButton("Copy All", this);
  m_copyBtn->setStyleSheet("padding: 2px 8px; font-size: 10px;");
  connect(m_copyBtn, &QPushButton::clicked, this,
          &DeveloperConsole::copyToClipboard);
  headerLayout->addWidget(m_copyBtn);

  m_clearBtn = new QPushButton("Clear", this);
  m_clearBtn->setStyleSheet("padding: 2px 8px; font-size: 10px;");
  connect(m_clearBtn, &QPushButton::clicked, this, &DeveloperConsole::clear);
  headerLayout->addWidget(m_clearBtn);

  mainLayout->addLayout(headerLayout);

  // Log View
  m_logView = new QTextEdit(this);
  m_logView->setReadOnly(true);
  m_logView->setStyleSheet(
      "QTextEdit { "
      "   background-color: #1e1e1e; "
      "   color: #e0e0e0; "
      "   font-family: 'Monaco', 'Courier New', monospace; "
      "   font-size: 11px; "
      "   border: 1px solid #333; "
      "}");
  mainLayout->addWidget(m_logView);

  // Initial height hint
  setMinimumHeight(150);
}

void DeveloperConsole::appendLog(const QString &message) {
  // Determine color based on log level
  QString color = "#e0e0e0";
  if (message.contains("[ERROR]"))
    color = "#ff6b6b";
  else if (message.contains("[WARN ]"))
    color = "#feca57";
  else if (message.contains("[DEBUG]"))
    color = "#54a0ff";
  else if (message.contains("[INFO ]"))
    color = "#1dd1a1";

  // Simple HTML formatting for color
  // Escape HTML special chars if needed, but for logs usually simple
  QString html =
      QString("<span style='color:%1'>%2</span>").arg(color, message);

  m_logView->append(html);

  // Auto scroll
  QScrollBar *sb = m_logView->verticalScrollBar();
  sb->setValue(sb->maximum());
}

void DeveloperConsole::clear() { m_logView->clear(); }

void DeveloperConsole::copyToClipboard() {
  QClipboard *clipboard = QApplication::clipboard();
  clipboard->setText(m_logView->toPlainText());
}

} // namespace Monitor3G
