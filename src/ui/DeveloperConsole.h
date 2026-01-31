#ifndef DEVELOPERCONSOLE_H
#define DEVELOPERCONSOLE_H

#include "../utils/LogMessenger.h"
#include <QHBoxLayout>
#include <QLabel>
#include <QPushButton>
#include <QTextEdit>
#include <QVBoxLayout>
#include <QWidget>

namespace Monitor3G {

class DeveloperConsole : public QWidget {
  Q_OBJECT
public:
  explicit DeveloperConsole(QWidget *parent = nullptr);

public slots:
  void appendLog(const QString &message);
  void clear();
  void copyToClipboard();

private:
  void setupUI();

  QTextEdit *m_logView;
  QPushButton *m_clearBtn;
  QPushButton *m_copyBtn;
  LogMessenger *m_messenger;
};

} // namespace Monitor3G

#endif // DEVELOPERCONSOLE_H
