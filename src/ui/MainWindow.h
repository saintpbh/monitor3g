#ifndef MAINWINDOW_H
#define MAINWINDOW_H

#include <QLabel>
#include <QList>
#include <QMainWindow>
#include <QPushButton>
#include <memory>

namespace Monitor3G {

class SourcePanel;
class PreviewWidget;
class ControlPanel;
class DeckLinkDevice;
class DeckLinkOutput;
class TestPatternSource;
class AbstractSource;
class DeveloperConsole;
class VideoController;

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
  void onTestPatternRequest(bool active);
  void onSourceAdded(const QString &type, const QString &path);
  void onSourceSelected(int index);

private:
  void setupUI();
  void createMenuBar();
  void createStatusBar();
  void updateWindowTitle();

  // Panels
  SourcePanel *m_sourcePanel;
  ControlPanel *m_controlPanel;
  DeveloperConsole *m_console;
  VideoController *m_videoController;

  // Monitors
  PreviewWidget *m_previewMonitor; // Left
  PreviewWidget *m_programMonitor; // Right

  // Cut/Auto Buttons
  QPushButton *m_cutBtn;
  QPushButton *m_autoBtn;

  // Status
  QLabel *m_statusLabel;
  QLabel *m_deviceLabel;
  QLabel *m_fpsLabel;

  // Core components
  std::unique_ptr<DeckLinkDevice> m_device;
  DeckLinkOutput *m_output;
  TestPatternSource *m_testPatternSource;
  AbstractSource *m_currentMainSource; // Track selected media source
  QList<AbstractSource *> m_sources;
};

} // namespace Monitor3G

#endif // MAINWINDOW_H
