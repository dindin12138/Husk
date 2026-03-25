#include "TerminalMachine.hpp"

namespace husk::daemon {

TerminalMachine::TerminalMachine(uint16_t cols, uint16_t rows)
    : m_terminal(cols, rows), m_current_cols(cols), m_current_rows(rows) {}

void TerminalMachine::feed_input(std::string_view bytes) {
  m_terminal.write(bytes);
}

void TerminalMachine::resize(uint16_t cols, uint16_t rows) {
  if (m_current_cols == cols && m_current_rows == rows)
    return;
  m_current_cols = cols;
  m_current_rows = rows;
  m_terminal.resize(cols, rows);
}

void TerminalMachine::snapshot_to_buffer(common::GridSnapshot *back_buffer) {
  if (!back_buffer)
    return;

  m_render_state.update(m_terminal);

  back_buffer->cursor_x = m_render_state.cursor_x();
  back_buffer->cursor_y = m_render_state.cursor_y();
  back_buffer->cursor_visible = m_render_state.is_cursor_visible();

  m_render_state.populate_iterator(m_row_iter);

  uint16_t current_row = 0;

  while (m_row_iter.next() && current_row < back_buffer->rows) {

    m_row_iter.populate_cells(m_row_cells);

    uint16_t current_col = 0;

    while (m_row_cells.next() && current_col < back_buffer->cols) {

      size_t index = current_row * back_buffer->cols + current_col;
      auto &husk_cell = back_buffer->cells[index];

      husk_cell.bg_color = m_row_cells.bg_color();
      husk_cell.fg_color = m_row_cells.fg_color();
      husk_cell.codepoint = m_row_cells.codepoint();

      current_col++;
    }
    current_row++;
  }
}

} // namespace husk::daemon
