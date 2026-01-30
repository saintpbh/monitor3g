#include "MainWindow.h"
#include "../core/DeckLinkDevice.h"
#include "../utils/Logger.h"
#include "ControlPanel.h"
#include "PreviewWidget.h"
#include "SourcePanel.h"

#include <QAction>
#include <QCloseEvent>
#include <QHBoxLayout>
#include <QMenu>
#include <QMenuBar>
#include <QMessageBox>
#include <QSplitter>
#include <QVBoxLayout>

namespace Monitor3G {

MainWindow::MainWindow(QWidget *parent)
    : QMainWindow(parent), m_sourcePanel(nullptr), m_previewWidget(nullptr),
      m_controlPanel(nullptr), m_statusLabel(nullptr), m_deviceLabel(nullptr),
      m_fpsLabel(nullptr) {
  setupUI();
  createMenuBar();
  createStatusBar();
  updateWindowTitle();

  // Initialize device
  m_device = std::make_unique<DeckLinkDevice>();

  LOG_INFO("MainWindow initialized");
}

MainWindow::~MainWindow() { LOG_INFO("MainWindow destroyed"); }

void MainWindow::setupUI() {
  // Set window properties
  setMinimumSize(1200, 700);
  resize(1400, 800);

  // Create central widget
  QWidget *centralWidget = new QWidget(this);
  setCentralWidget(centralWidget);

  // Create main layout
  QHBoxLayout *mainLayout = new QHBoxLayout(centralWidget);
  mainLayout->setContentsMargins(4, 4, 4, 4);
  mainLayout->setSpacing(4);

  // Create splitter
  QSplitter *splitter = new QSplitter(Qt::Horizontal, this);

  // Create source panel (left side)
  m_sourcePanel = new SourcePanel(this);
  m_sourcePanel->setMinimumWidth(200);
  m_sourcePanel->setMaximumWidth(300);
  splitter->addWidget(m_sourcePanel);

  // Create right side container
  QWidget *rightContainer = new QWidget(this);
  QVBoxLayout *rightLayout = new QVBoxLayout(rightContainer);
  rightLayout->setContentsMargins(0, 0, 0, 0);
  rightLayout->setSpacing(4);

  // Create preview widget
  m_previewWidget = new PreviewWidget(this);
  rightLayout->addWidget(m_previewWidget, 1);

  // Create control panel
  m_controlPanel = new ControlPanel(this);
  m_controlPanel->setMaximumHeight(150);
  rightLayout->addWidget(m_controlPanel);

  splitter->addWidget(rightContainer);

  // Set splitter proportions
  splitter->setStretchFactor(0, 0);
  splitter->setStretchFactor(1, 1);

  mainLayout->addWidget(splitter);
}

void MainWindow::createMenuBar() {
  // File menu
  QMenu *fileMenu = menuBar()->addMenu(tr("&File"));

  QAction *openAction = fileMenu->addAction(tr("&Open Source..."));
  openAction->setShortcut(QKeySequence::Open);

  fileMenu->addSeparator();

  QAction *quitAction = fileMenu->addAction(tr("&Quit"));
  quitAction->setShortcut(QKeySequence::Quit);
  connect(quitAction, &QAction::triggered, this, &QMainWindow::close);

  // Sources menu
  QMenu *sourcesMenu = menuBar()->addMenu(tr("&Sources"));
  sourcesMenu->addAction(tr("Video File..."));
  sourcesMenu->addAction(tr("Image..."));
  sourcesMenu->addAction(tr("PDF Document..."));
  sourcesMenu->addAction(tr("Screen Capture..."));
  sourcesMenu->addAction(tr("Live Camera..."));

  // Output menu
  QMenu *outputMenu = menuBar()->addMenu(tr("&Output"));
  outputMenu->addAction(tr("Device Settings..."));
  outputMenu->addAction(tr("Format Settings..."));
  outputMenu->addSeparator();
  outputMenu->addAction(tr("Start Output"));
  outputMenu->addAction(tr("Stop Output"));

  // Help menu
  QMenu *helpMenu = menuBar()->addMenu(tr("&Help"));
  helpMenu->addAction(tr("&About"));
  helpMenu->addAction(tr("Documentation"));
}

void MainWindow::createStatusBar() {
  // Create status bar widgets
  m_statusLabel = new QLabel(tr("Ready"), this);
  m_deviceLabel = new QLabel(tr("No Device"), this);
  m_fpsLabel = new QLabel(tr("0 fps"), this);

  // Add to status bar
  statusBar()->addWidget(m_statusLabel, 1);
  statusBar()->addPermanentWidget(m_deviceLabel);
  statusBar()->addPermanentWidget(m_fpsLabel);

  LOG_DEBUG("Status bar created");
}

void MainWindow::updateWindowTitle() {
  setWindowTitle(tr("Monitor3G - Blackmagic Output Control"));
}

void MainWindow::closeEvent(QCloseEvent *event) {
  LOG_INFO("Application closing");

  // TODO: Check if output is running and confirm

  event->accept();
}

void MainWindow::onDeviceStatusChanged(bool connected) {
  if (connected) {
    m_deviceLabel->setText(tr("Device Connected"));
    m_statusLabel->setText(tr("Ready to output"));
  } else {
    m_deviceLabel->setText(tr("No Device"));
    m_statusLabel->setText(tr("No output device available"));
  }
}

void MainWindow::onOutputStarted() {
  m_statusLabel->setText(tr("● LIVE"));
  LOG_INFO("Output started");
}

void MainWindow::onOutputStopped() {
  m_statusLabel->setText(tr("Ready"));
  LOG_INFO("Output stopped");
}

} // namespace Monitor3G
