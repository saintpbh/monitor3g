#include "MainWindow.h"
#include "../core/DeckLinkDevice.h"
#include "../core/DeckLinkOutput.h"
#include "../sources/TestPatternSource.h"
#include "../utils/Logger.h"
#include "ControlPanel.h"
#include "DeveloperConsole.h"
#include "PreviewWidget.h"
#include "SourcePanel.h"

#include <QAction>
#include <QCloseEvent>
#include <QHBoxLayout>
#include <QMenuBar>
#include <QPushButton>
#include <QSplitter>
#include <QStatusBar>
#include <QVBoxLayout>

namespace Monitor3G {

MainWindow::MainWindow(QWidget *parent)
    : QMainWindow(parent), m_sourcePanel(nullptr), m_controlPanel(nullptr),
      m_console(nullptr), m_previewMonitor(nullptr), m_programMonitor(nullptr),
      m_output(nullptr), m_testPatternSource(nullptr),
      m_currentMainSource(nullptr) {

  // Dark Theme
  setStyleSheet("QMainWindow { background-color: #2b2b2b; color: #e0e0e0; }");

  setupUI();
  createMenuBar();
  createStatusBar();
  updateWindowTitle();

  // Initialize Core Systems
  m_device = std::make_unique<DeckLinkDevice>();
  m_output = new DeckLinkOutput(this);
  m_testPatternSource = new TestPatternSource(this);
  m_testPatternSource->setPattern(TestPattern::ColorBars); // Default to bars

  // Try to open first DeckLink device
  if (m_device->openDevice(0)) {
    LOG_INFO("Opened DeckLink device: " + m_device->getDeviceName());
    if (m_output->initialize(m_device->getDeckLinkInterface())) {
      LOG_INFO("DeckLink Output initialized successfully");
      m_deviceLabel->setText("DEVICE: " + m_device->getDeviceName());
      m_deviceLabel->setStyleSheet("color: #00ff00;"); // Green for OK
    } else {
      LOG_ERROR("Failed to initialize DeckLink Output");
      m_deviceLabel->setText("DEVICE: ERROR");
      m_deviceLabel->setStyleSheet("color: #ff0000;");
    }
  } else {
    LOG_WARNING("No DeckLink device found. Running in Simulation Mode.");
    m_deviceLabel->setText("DEVICE: SIMULATION");
    m_deviceLabel->setStyleSheet("color: #ffff00;"); // Yellow for Sim
    m_deviceLabel->setStyleSheet("color: #ffff00;"); // Yellow for Sim
  }

  // Monitor Connections
  connect(m_output, &DeckLinkOutput::videoFrameArrived, m_previewMonitor,
          &PreviewWidget::setFrame);
  connect(m_output, &DeckLinkOutput::videoFrameArrived, m_programMonitor,
          &PreviewWidget::setFrame);

  LOG_INFO("MainWindow initialized - Hardware Console Mode");
}

MainWindow::~MainWindow() {
  if (m_output) {
    m_output->stop();
  }
  if (m_device) {
    m_device->closeDevice();
  }
  LOG_INFO("MainWindow destroyed");
}

void MainWindow::setupUI() {
  setMinimumSize(1280, 800);
  resize(1400, 900);

  QWidget *centralWidget = new QWidget(this);
  setCentralWidget(centralWidget);
  QVBoxLayout *mainVLayout = new QVBoxLayout(centralWidget);
  mainVLayout->setContentsMargins(0, 0, 0, 0);
  mainVLayout->setSpacing(0);

  // Top Area (Splitter: Sources | Monitors)
  QSplitter *topSplitter = new QSplitter(Qt::Horizontal, this);
  topSplitter->setStyleSheet("QSplitter::handle { background-color: #111; }");

  // Left: Media Pool
  QWidget *leftContainer = new QWidget(this);
  QVBoxLayout *leftLayout = new QVBoxLayout(leftContainer);
  leftLayout->setContentsMargins(0, 0, 0, 0);
  leftLayout->setSpacing(0);

  QLabel *poolLabel = new QLabel("   MEDIA POOL / SOURCES", this);
  poolLabel->setStyleSheet("background-color: #1a1a1a; color: #888; "
                           "font-weight: bold; font-size: 10px; padding: 4px;");
  leftLayout->addWidget(poolLabel);

  m_sourcePanel = new SourcePanel(this);
  leftLayout->addWidget(m_sourcePanel);

  topSplitter->addWidget(leftContainer);
  topSplitter->setStretchFactor(0, 1);

  // Right: Monitors area
  QWidget *monitorsContainer = new QWidget(this);
  QVBoxLayout *monitorsLayout = new QVBoxLayout(monitorsContainer);
  monitorsLayout->setContentsMargins(4, 4, 4, 4);

  QLabel *monitorLabel =
      new QLabel("Monitor3G - Professional Hardware Console", this);
  monitorLabel->setAlignment(Qt::AlignCenter);
  monitorLabel->setStyleSheet(
      "color: #888; font-size: 11px; font-weight: bold; padding: 4px;");
  monitorsLayout->addWidget(monitorLabel);

  QHBoxLayout *dualMonitorLayout = new QHBoxLayout();

  // Preview Monitor (Green)
  QWidget *previewContainer = new QWidget(this);
  QVBoxLayout *previewLayout = new QVBoxLayout(previewContainer);
  previewLayout->setContentsMargins(0, 0, 0, 0);
  previewLayout->setSpacing(0);
  m_previewMonitor = new PreviewWidget(this);
  m_previewMonitor->setStyleSheet(
      "border: 1px solid #333; background-color: black;");
  QLabel *pvwLabel = new QLabel("PREVIEW", this);
  pvwLabel->setAlignment(Qt::AlignCenter);
  pvwLabel->setStyleSheet("background-color: #00aa44; color: white; "
                          "font-weight: bold; font-size: 10px;");
  previewLayout->addWidget(m_previewMonitor, 1);
  previewLayout->addWidget(pvwLabel);

  // Program Monitor (Red)
  QWidget *programContainer = new QWidget(this);
  QVBoxLayout *programLayout = new QVBoxLayout(programContainer);
  programLayout->setContentsMargins(0, 0, 0, 0);
  programLayout->setSpacing(0);
  m_programMonitor = new PreviewWidget(this);
  m_programMonitor->setStyleSheet(
      "border: 1px solid #333; background-color: black;");
  QLabel *pgmLabel = new QLabel("PROGRAM", this);
  pgmLabel->setAlignment(Qt::AlignCenter);
  pgmLabel->setStyleSheet("background-color: #cc2200; color: white; "
                          "font-weight: bold; font-size: 10px;");
  programLayout->addWidget(m_programMonitor, 1);
  programLayout->addWidget(pgmLabel);

  // Center Buttons (Cut/Auto)
  QVBoxLayout *centerBtnLayout = new QVBoxLayout();
  centerBtnLayout->addStretch();
  m_cutBtn = new QPushButton("CUT", this);
  m_cutBtn->setMinimumSize(60, 40);
  m_cutBtn->setStyleSheet("background-color: #444; color: white; border: 1px "
                          "solid #666; border-radius: 4px; font-weight: bold;");

  m_autoBtn = new QPushButton("AUTO", this);
  m_autoBtn->setMinimumSize(60, 40);
  m_autoBtn->setStyleSheet(
      "background-color: #444; color: white; border: 1px solid #666; "
      "border-radius: 4px; font-weight: bold;");

  centerBtnLayout->addWidget(m_cutBtn);
  centerBtnLayout->addWidget(m_autoBtn);
  centerBtnLayout->addStretch();

  dualMonitorLayout->addWidget(previewContainer, 1);
  dualMonitorLayout->addLayout(centerBtnLayout, 0);
  dualMonitorLayout->addWidget(programContainer, 1);

  monitorsLayout->addLayout(dualMonitorLayout);

  // Control Panel at bottom of Monitor area
  m_controlPanel = new ControlPanel(this);
  m_controlPanel->setMaximumHeight(200);
  monitorsLayout->addWidget(m_controlPanel);

  topSplitter->addWidget(monitorsContainer);
  topSplitter->setStretchFactor(1, 4);

  mainVLayout->addWidget(topSplitter, 1); // Expandable upper area

  // Bottom Area: Developer Console
  m_console = new DeveloperConsole(this);
  m_console->setMaximumHeight(180);
  mainVLayout->addWidget(m_console, 0); // Fixed size

  // Wire up Control Panel
  connect(m_controlPanel, &ControlPanel::startOutputRequested, this,
          &MainWindow::onOutputStarted);
  connect(m_controlPanel, &ControlPanel::stopOutputRequested, this,
          &MainWindow::onOutputStopped);
  connect(m_controlPanel, &ControlPanel::testPatternRequested, this,
          &MainWindow::onTestPatternRequest);
}

void MainWindow::createMenuBar() {
  QMenu *fileMenu = menuBar()->addMenu(tr("&File"));
  fileMenu->addAction("Quit", this, &QWidget::close);
}

void MainWindow::createStatusBar() {
  m_statusLabel = new QLabel("READY", this);
  m_statusLabel->setStyleSheet("color: #aaa; padding-left: 5px;");

  m_deviceLabel = new QLabel("DEVICE: UNKNOWN", this);
  m_deviceLabel->setStyleSheet("color: #888; padding-right: 10px;");

  m_fpsLabel = new QLabel("FPS: 0", this);
  m_fpsLabel->setStyleSheet("color: #888;");

  statusBar()->addWidget(m_statusLabel, 1);
  statusBar()->addPermanentWidget(m_deviceLabel);
  statusBar()->addPermanentWidget(m_fpsLabel);
}

void MainWindow::updateWindowTitle() {
  setWindowTitle("Monitor3G - Professional Hardware Console");
}

void MainWindow::closeEvent(QCloseEvent *event) {
  LOG_INFO("Application closing");
  event->accept();
}

void MainWindow::onDeviceStatusChanged(bool connected) {}

void MainWindow::onOutputStarted() {
  if (m_output) {
    m_output->start();
  }
}

void MainWindow::onOutputStopped() {
  if (m_output) {
    m_output->stop();
  }
}

void MainWindow::onTestPatternRequest(bool active) {
  if (active) {
    LOG_INFO("Output source switched to: Test Pattern");
    if (m_output) {
      m_output->setSource(m_testPatternSource);
    }
  } else {
    LOG_INFO("Output source restored to: Media Pool Selection");
    // For now, restoring to nothing if no main source selected.
    // In future, we restore m_currentMainSource
    if (m_output) {
      m_output->setSource(m_currentMainSource);
    }
  }
}

} // namespace Monitor3G
