#ifndef DEBUGCONSOLE_H
#define DEBUGCONSOLE_H

#include "../utils/LogMessenger.h"
#include <QWidget>

class QPlainTextEdit;
class QPushButton;

namespace Monitor3G {

class DebugConsole : public QWidget {
  Q_OBJECT

public:
  explicit DebugConsole(QWidget *parent = nullptr);

public slots:
  void appendLog(const QString &message, Monitor3G::LogLevel level);
  void clear();
  void copyAll();

private:
  void setupUI();

  QPlainTextEdit *m_textEdit;
  QPushButton *m_copyBtn;
  QPushButton *m_clearBtn;
};

} // namespace Monitor3G

#endif // DEBUGCONSOLE_H
