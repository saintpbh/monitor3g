#include "Style.h"

namespace Monitor3G {

QString Style::getDarkTheme() {
  return R"(
        QMainWindow {
            background-color: #1e1e1e;
            color: #ffffff;
        }
        
        QWidget {
            background-color: #2b2b2b;
            color: #dddddd;
            font-family: "Segoe UI", "Helvetica Neue", Arial, sans-serif;
            font-size: 14px;
        }

        /* Splitter */
        QSplitter::handle {
            background-color: #444;
        }
        QSplitter::handle:hover {
            background-color: #666;
        }

        /* Buttons */
        QPushButton {
            background-color: #444;
            border: 1px solid #555;
            border-radius: 4px;
            padding: 5px 15px;
            color: white;
        }
        QPushButton:hover {
            background-color: #555;
            border-color: #666;
        }
        QPushButton:pressed {
            background-color: #333;
            border-color: #444;
        }
        QPushButton:disabled {
            background-color: #2a2a2a;
            color: #666;
            border-color: #333;
        }

        /* Labels */
        QLabel {
            color: #cccccc;
        }

        /* Lists and Trees */
        QListWidget {
            background-color: #222;
            border: 1px solid #333;
            border-radius: 4px;
            color: #eee;
        }
        QListWidget::item {
            padding: 5px;
        }
        QListWidget::item:selected {
            background-color: #3d5afe; /* Accent Color */
            color: white;
        }
        QListWidget::item:hover {
            background-color: #333;
        }

        /* Combo Box */
        QComboBox {
            background-color: #333;
            border: 1px solid #444;
            border-radius: 4px;
            padding: 4px;
            color: white;
        }
        QComboBox:hover {
            border-color: #555;
        }
        QComboBox::drop-down {
            border: none;
        }

        /* Scrollbars */
        QScrollBar:vertical {
            background: #2b2b2b;
            width: 10px;
            margin: 0px;
        }
        QScrollBar::handle:vertical {
            background: #555;
            min-height: 20px;
            border-radius: 5px;
        }
        QScrollBar::handle:vertical:hover {
            background: #777;
        }
        QScrollBar::add-line:vertical, QScrollBar::sub-line:vertical {
            background: none;
        }

        /* Slider */
        QSlider::groove:horizontal {
            border: 1px solid #333;
            height: 4px;
            background: #222;
            margin: 2px 0;
            border-radius: 2px;
        }
        QSlider::handle:horizontal {
            background: #ccc;
            border: 1px solid #ccc;
            width: 12px;
            height: 12px;
            margin: -4px 0;
            border-radius: 6px;
        }
        QSlider::handle:horizontal:hover {
            background: white;
        }
        
        /* Tally Colors handled in code but defaults here */
        .preview-tally { border: 2px solid #4CAF50; }
        .program-tally { border: 2px solid #F44336; }
    )";
}

QString Style::getLightTheme() {
  return R"(
        QMainWindow {
            background-color: #f0f0f0;
            color: #333333;
        }
        
        QWidget {
            background-color: #ffffff;
            color: #333333;
            font-family: -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, "Helvetica Neue", Arial, sans-serif;
            font-size: 14px;
        }

        /* Splitter */
        QSplitter::handle {
            background-color: #cccccc;
        }
        QSplitter::handle:hover {
            background-color: #bbbbbb;
        }

        /* Buttons */
        QPushButton {
            background-color: #007aff;
            border: 1px solid #007aff;
            border-radius: 8px;
            padding: 8px 16px;
            color: white;
            font-weight: 500;
        }
        QPushButton:hover {
            background-color: #005ecb;
            border-color: #005ecb;
        }
        QPushButton:pressed {
            background-color: #004a9e;
            border-color: #004a9e;
        }
        QPushButton:disabled {
            background-color: #e0e0e0;
            color: #9e9e9e;
            border-color: #e0e0e0;
        }

        /* Labels */
        QLabel {
            color: #333333;
        }

        /* Lists and Trees */
        QListWidget {
            background-color: #ffffff;
            border: 1px solid #cccccc;
            border-radius: 8px;
            color: #333333;
        }
        QListWidget::item {
            padding: 8px;
        }
        QListWidget::item:selected {
            background-color: #007aff; /* Accent Color */
            color: white;
        }
        QListWidget::item:hover {
            background-color: #f0f0f0;
        }

        /* Combo Box */
        QComboBox {
            background-color: #ffffff;
            border: 1px solid #cccccc;
            border-radius: 8px;
            padding: 6px;
            color: #333333;
        }
        QComboBox:hover {
            border-color: #bbbbbb;
        }
        QComboBox::drop-down {
            border: none;
        }

        /* Scrollbars */
        QScrollBar:vertical {
            background: #f0f0f0;
            width: 12px;
            margin: 0px;
        }
        QScrollBar::handle:vertical {
            background: #cccccc;
            min-height: 24px;
            border-radius: 6px;
        }
        QScrollBar::handle:vertical:hover {
            background: #bbbbbb;
        }
        QScrollBar::add-line:vertical, QScrollBar::sub-line:vertical {
            background: none;
        }

        /* Slider */
        QSlider::groove:horizontal {
            border: 1px solid #cccccc;
            height: 6px;
            background: #e0e0e0;
            margin: 2px 0;
            border-radius: 3px;
        }
        QSlider::handle:horizontal {
            background: #007aff;
            border: 1px solid #007aff;
            width: 16px;
            height: 16px;
            margin: -5px 0;
            border-radius: 8px;
        }
        QSlider::handle:horizontal:hover {
            background: #005ecb;
        }
        
        /* Tally Colors handled in code but defaults here */
        .preview-tally { border: 3px solid #4CAF50; }
        .program-tally { border: 3px solid #F44336; }
    )";
}

} // namespace Monitor3G
