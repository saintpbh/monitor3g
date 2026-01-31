#include "ProgramPreviewWidget.h"
#include <QFrame>

namespace Monitor3G {

ProgramPreviewWidget::ProgramPreviewWidget(QWidget *parent)
    : QWidget(parent), m_previewMonitor(nullptr), m_programMonitor(nullptr) {
  setupUI();
}

void ProgramPreviewWidget::setupUI() {
  QHBoxLayout *mainLayout = new QHBoxLayout(this);
  mainLayout->setSpacing(20);
  mainLayout->setContentsMargins(10, 10, 10, 10);

  // No hardcoded background, let MainWindow's global style handle it
  // But we want a dark monitor area
  setObjectName("programPreviewLayout");
  setStyleSheet("QWidget#programPreviewLayout { background-color: #000000; }");

  // Initialize Monitors
  m_previewMonitor = new PreviewWidget(this);
  m_programMonitor = new PreviewWidget(this);

  // Create Monitor Groups with Tally
  QWidget *previewGroup =
      createMonitorGroup("PREVIEW", m_previewMonitor, "#4CAF50"); // Green
  QWidget *programGroup =
      createMonitorGroup("PROGRAM", m_programMonitor, "#F44336"); // Red

  // Center Controls
  QVBoxLayout *controlsLayout = new QVBoxLayout();
  controlsLayout->addStretch();

  m_cutBtn = new QPushButton("CUT", this);
  m_cutBtn->setMinimumSize(80, 40);
  m_cutBtn->setStyleSheet("QPushButton { background-color: #555; border: none; "
                          "border-radius: 4px; font-weight: bold; }"
                          "QPushButton:hover { background-color: #666; }"
                          "QPushButton:pressed { background-color: #444; }");
  connect(m_cutBtn, &QPushButton::clicked, this,
          &ProgramPreviewWidget::onCutClicked);
  controlsLayout->addWidget(m_cutBtn);

  m_fadeBtn = new QPushButton("AUTO", this);
  m_fadeBtn->setMinimumSize(80, 40);
  m_fadeBtn->setStyleSheet("QPushButton { background-color: #555; border: "
                           "none; border-radius: 4px; font-weight: bold; }"
                           "QPushButton:hover { background-color: #666; }"
                           "QPushButton:pressed { background-color: #444; }");
  connect(m_fadeBtn, &QPushButton::clicked, this,
          &ProgramPreviewWidget::onFadeClicked);
  controlsLayout->addWidget(m_fadeBtn);

  controlsLayout->addStretch();

  // Assemble Layout
  mainLayout->addWidget(previewGroup, 1);
  mainLayout->addLayout(controlsLayout, 0);
  mainLayout->addWidget(programGroup, 1);
}

QWidget *ProgramPreviewWidget::createMonitorGroup(const QString &title,
                                                  PreviewWidget *monitor,
                                                  const QString &tallyColor) {
  QWidget *group = new QWidget(this);
  QVBoxLayout *layout = new QVBoxLayout(group);
  layout->setContentsMargins(0, 0, 0, 0);
  layout->setSpacing(5);

  layout->addWidget(monitor);

  // Tally Bar / Title
  QLabel *titleLabel = new QLabel(title, group);
  titleLabel->setAlignment(Qt::AlignCenter);
  titleLabel->setStyleSheet(QString("background-color: %1; color: white; "
                                    "font-family: 'Inter', sans-serif; "
                                    "font-size: 10px; font-weight: 800; "
                                    "padding: 2px; border-radius: 0px;")
                                .arg(tallyColor));
  layout->addWidget(titleLabel);

  // Add distinct border to monitor
  monitor->setStyleSheet(
      QString("PreviewWidget { border: 1px solid %1; background: #050505; }")
          .arg(tallyColor));

  return group;
}

void ProgramPreviewWidget::setPreviewFrame(const QImage &frame) {
  if (m_previewMonitor) {
    m_previewMonitor->setFrame(frame);
  }
}

void ProgramPreviewWidget::setProgramFrame(const QImage &frame) {
  if (m_programMonitor) {
    m_programMonitor->setFrame(frame);
  }
}

void ProgramPreviewWidget::onCutClicked() { emit cutRequested(); }

void ProgramPreviewWidget::onFadeClicked() { emit fadeRequested(); }

} // namespace Monitor3G
