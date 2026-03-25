#pragma once

#include <atomic>
#include <cstdint>
#include <memory>

namespace husk::common {

// Buffer
struct alignas(8) Cell {
  uint32_t codepoint;
  uint32_t fg_color;
  uint32_t bg_color;
  uint16_t flags;
};

struct GridSnapshot {
  uint16_t cols;
  uint16_t rows;
  uint16_t cursor_x;
  uint16_t cursor_y;
  bool cursor_visible;

  std::unique_ptr<Cell[]> cells;

  GridSnapshot(uint16_t c, uint16_t r)
      : cols(c), rows(r), cursor_x(0), cursor_y(0), cursor_visible(true),
        cells(std::make_unique<Cell[]>(c * r)) {
    for (size_t i = 0; i < c * r; ++i) {
      cells[i].codepoint = 0;
      cells[i].fg_color = 0xFFFFFFFF;
      cells[i].bg_color = 0x000000FF;
      cells[i].flags = 0;
    }
  }
};

class SharedState {
private:
  std::atomic<GridSnapshot *> m_idle{nullptr};
  std::atomic<bool> m_has_new_frame{false};

  GridSnapshot *m_front{nullptr};
  GridSnapshot *m_back{nullptr};

public:
  SharedState(uint16_t cols, uint16_t rows) {
    m_front = new GridSnapshot(cols, rows);
    m_idle.store(new GridSnapshot(cols, rows), std::memory_order_relaxed);
    m_back = new GridSnapshot(cols, rows);
  }

  ~SharedState() {
    delete m_front;
    delete m_back;
    delete m_idle.load(std::memory_order_relaxed);
  }

  SharedState(const SharedState &) = delete;
  SharedState &operator=(const SharedState &) = delete;

  GridSnapshot *get_back_buffer_for_write(uint16_t current_cols,
                                          uint16_t current_rows) {
    if (m_back->cols != current_cols || m_back->rows != current_rows) {
      delete m_back;
      m_back = new GridSnapshot(current_cols, current_rows);
    }
    return m_back;
  }

  void commit_back_buffer() {
    m_back = m_idle.exchange(m_back, std::memory_order_acq_rel);
    m_has_new_frame.store(true, std::memory_order_release);
  }

  GridSnapshot *acquire_front_buffer() {
    if (m_has_new_frame.exchange(false, std::memory_order_acquire)) {
      m_front = m_idle.exchange(m_front, std::memory_order_acq_rel);
    }
    return m_front;
  }
};

} // namespace husk::common
