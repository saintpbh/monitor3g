#ifndef PROGRAMPREVIEWWIDGET_H
#define PROGRAMPREVIEWWIDGET_H

#include "PreviewWidget.h"
#include <QHBoxLayout>
#include <QLabel>
#include <QPushButton>
#include <QVBoxLayout>
#include <QWidget>

namespace Monitor3G {

class ProgramPreviewWidget : public QWidget {
  Q_OBJECT

public:
  explicit ProgramPreviewWidget(QWidget *parent = nullptr);

  // Update the frames for respective monitors
  void setPreviewFrame(const QImage &frame);
  void setProgramFrame(const QImage &frame);

signals:
  void cutRequested();
  void fadeRequested();

private slots:
  void onCutClicked();
  void onFadeClicked();

private:
  void setupUI();
  QWidget *createMonitorGroup(const QString &title, PreviewWidget *monitor,
                              const QString &tallyColor);

  PreviewWidget *m_previewMonitor;
  PreviewWidget *m_programMonitor;

  QPushButton *m_cutBtn;
  QPushButton *m_fadeBtn;
};

} // namespace Monitor3G

#endif // PROGRAMPREVIEWWIDGET_H
