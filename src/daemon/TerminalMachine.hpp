#pragma once

#include "common/SharedGrid.hpp"
#include "vt/RenderState.hpp"
#include "vt/Terminal.hpp"
#include <string_view>

namespace husk::daemon {

class TerminalMachine {
public:
  TerminalMachine(uint16_t cols, uint16_t rows);
  ~TerminalMachine() = default;

  TerminalMachine(const TerminalMachine &) = delete;
  TerminalMachine &operator=(const TerminalMachine &) = delete;

  void feed_input(std::string_view bytes);
  void resize(uint16_t cols, uint16_t rows);
  void snapshot_to_buffer(common::GridSnapshot *back_buffer);

private:
  husk::vt::Terminal m_terminal;

  husk::vt::RenderState m_render_state;
  husk::vt::RowIterator m_row_iter;
  husk::vt::RowCells m_row_cells;

  uint16_t m_current_cols;
  uint16_t m_current_rows;
};

} // namespace husk::daemon
