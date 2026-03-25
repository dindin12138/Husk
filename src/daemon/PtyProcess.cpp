#include "PtyProcess.hpp"

#include <cstdlib>
#include <cstring>
#include <fcntl.h>
#include <signal.h>
#include <stdexcept>
#include <sys/wait.h>
#include <unistd.h>

#if defined(__linux__)
#include <pty.h>
#include <sys/eventfd.h>
#elif defined(__APPLE__)
#include <util.h>
#endif

namespace husk::daemon {

PtyProcess::PtyProcess(uint16_t cols, uint16_t rows) {
#if defined(__linux__)
  m_wakeup_fd = eventfd(0, EFD_NONBLOCK | EFD_CLOEXEC);
  if (m_wakeup_fd == -1) {
    throw std::runtime_error("Failed to create eventfd for wakeup mechanism.");
  }
#endif

  struct winsize ws{};
  ws.ws_col = cols;
  ws.ws_row = rows;

  m_pid = forkpty(&m_pty_fd, nullptr, nullptr, &ws);

  if (m_pid == -1) {
    throw std::runtime_error("forkpty failed! Cannot spawn shell.");
  } else if (m_pid == 0) {
    setenv("TERM", "xterm-256color", 1);
    setenv("COLORTERM", "truecolor", 1);

    const char *shell = getenv("SHELL");
    if (!shell)
      shell = "sh";

    execlp(shell, shell, "-c", "ls -la", nullptr);

    perror("[Husk Child] execlp failed");
    exit(1);
  }

  int flags = fcntl(m_pty_fd, F_GETFL, 0);
  if (flags != -1) {
    fcntl(m_pty_fd, F_SETFL, flags | O_NONBLOCK);
  }
}

PtyProcess::~PtyProcess() {
  if (m_pid > 0) {
    kill(m_pid, SIGHUP);
    int status;
    waitpid(m_pid, &status, 0);
  }

  if (m_pty_fd != -1)
    close(m_pty_fd);
  if (m_wakeup_fd != -1)
    close(m_wakeup_fd);
}

void PtyProcess::enqueue_command(DaemonCommand cmd) {
  bool needs_wakeup = false;
  {
    std::lock_guard<std::mutex> lock(m_cmd_mutex);
    m_cmd_queue.push_back(std::move(cmd));

    if (m_is_sleeping) {
      m_is_sleeping = false;
      needs_wakeup = true;
    }
  }

#if defined(__linux__)
  if (needs_wakeup && m_wakeup_fd != -1) {
    uint64_t val = 1;
    if (write(m_wakeup_fd, &val, sizeof(val)) == -1) {
      // handle error strictly if needed
    }
  }
#endif
}

std::deque<DaemonCommand> PtyProcess::acquire_all_commands() {
  std::lock_guard<std::mutex> lock(m_cmd_mutex);

  std::deque<DaemonCommand> stolen_queue;
  stolen_queue.swap(m_cmd_queue);

  m_is_sleeping = true;

  return stolen_queue;
}

} // namespace husk::daemon
