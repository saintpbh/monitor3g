#ifndef MAINWINDOW_H
#define MAINWINDOW_H

#include <QComboBox>
#include <QLabel>
#include <QMainWindow>
#include <QPushButton>
#include <QStatusBar>
#include <memory>

namespace Monitor3G {

class SourcePanel;
class PreviewWidget;
class ControlPanel;
class DeckLinkDevice;

class MainWindow : public QMainWindow {
  Q_OBJECT

public:
  explicit MainWindow(QWidget *parent = nullptr);
  ~MainWindow();

protected:
  void closeEvent(QCloseEvent *event) override;

private slots:
  void onDeviceStatusChanged(bool connected);
  void onOutputStarted();
  void onOutputStopped();

private:
  void setupUI();
  void createMenuBar();
  void createStatusBar();
  void updateWindowTitle();

  // UI Components
  SourcePanel *m_sourcePanel;
  PreviewWidget *m_previewWidget;
  ControlPanel *m_controlPanel;

  // Status bar widgets
  QLabel *m_statusLabel;
  QLabel *m_deviceLabel;
  QLabel *m_fpsLabel;

  // Core components
  std::unique_ptr<DeckLinkDevice> m_device;
};

} // namespace Monitor3G

#endif // MAINWINDOW_H
