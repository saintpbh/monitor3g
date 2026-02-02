#ifndef SOURCEPANEL_H
#define SOURCEPANEL_H

#include <QDragEnterEvent>
#include <QDropEvent>
#include <QListWidget>
#include <QMimeData>
#include <QPushButton>
#include <QWidget>

namespace Monitor3G {

class SourcePanel : public QWidget {
  Q_OBJECT

public:
  explicit SourcePanel(QWidget *parent = nullptr);

signals:
  void sourceSelected(int sourceIndex);
  void sourceAdded(const QString &path, const QString &type);

protected:
  void dragEnterEvent(QDragEnterEvent *event) override;
  void dropEvent(QDropEvent *event) override;

private slots:
  void onAddVideoClicked();
  void onAddImageClicked();
  void onAddPDFClicked();
  void onAddScreenCaptureClicked();
  void onAddLiveCameraClicked();

private:
  void setupUI();

  QListWidget *m_sourceList;
  QPushButton *m_addVideoBtn;
  QPushButton *m_addImageBtn;
  QPushButton *m_addPDFBtn;
  QPushButton *m_addScreenBtn;
  QPushButton *m_addCameraBtn;
};

} // namespace Monitor3G

#endif // SOURCEPANEL_H
