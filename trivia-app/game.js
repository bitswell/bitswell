(() => {
  let allQuestions = [];
  let gameQuestions = [];
  let currentIndex = 0;
  let score = 0;
  let streak = 0;
  let bestStreak = 0;
  let totalTime = 0;
  let timerInterval = null;
  let timeLeft = 30;
  let answered = false;
  let selectedCategories = new Set();
  let selectedDifficulty = "all";
  let questionCount = 20;

  const TIMER_SECONDS = 30;
  const LETTERS = ["A", "B", "C", "D"];

  // DOM refs
  const screens = {
    start: document.getElementById("screen-start"),
    game: document.getElementById("screen-game"),
    results: document.getElementById("screen-results"),
  };

  function showScreen(name) {
    Object.values(screens).forEach((s) => s.classList.remove("active"));
    screens[name].classList.add("active");
  }

  // Load questions
  async function loadQuestions() {
    const resp = await fetch("questions.json");
    allQuestions = await resp.json();
    initCategoryButtons();
  }

  function initCategoryButtons() {
    const categories = [...new Set(allQuestions.map((q) => q.category))];
    const container = document.getElementById("category-buttons");
    categories.forEach((cat) => {
      selectedCategories.add(cat);
      const btn = document.createElement("button");
      btn.className = "cat-btn active";
      btn.textContent = cat;
      btn.addEventListener("click", () => {
        if (selectedCategories.has(cat)) {
          selectedCategories.delete(cat);
          btn.classList.remove("active");
        } else {
          selectedCategories.add(cat);
          btn.classList.add("active");
        }
      });
      container.appendChild(btn);
    });
  }

  // Difficulty buttons
  document.querySelectorAll(".diff-btn").forEach((btn) => {
    btn.addEventListener("click", () => {
      document.querySelectorAll(".diff-btn").forEach((b) => b.classList.remove("active"));
      btn.classList.add("active");
      selectedDifficulty = btn.dataset.diff;
    });
  });

  // Count buttons
  document.querySelectorAll(".count-btn").forEach((btn) => {
    btn.addEventListener("click", () => {
      document.querySelectorAll(".count-btn").forEach((b) => b.classList.remove("active"));
      btn.classList.add("active");
      questionCount = parseInt(btn.dataset.count);
    });
  });

  // Shuffle array (Fisher-Yates)
  function shuffle(arr) {
    for (let i = arr.length - 1; i > 0; i--) {
      const j = Math.floor(Math.random() * (i + 1));
      [arr[i], arr[j]] = [arr[j], arr[i]];
    }
    return arr;
  }

  // Start game
  document.getElementById("btn-start").addEventListener("click", () => {
    if (selectedCategories.size === 0) return;

    let pool = allQuestions.filter((q) => selectedCategories.has(q.category));
    if (selectedDifficulty !== "all") {
      pool = pool.filter((q) => q.difficulty === selectedDifficulty);
    }

    if (pool.length === 0) return;

    shuffle(pool);
    gameQuestions = questionCount === 0 ? pool : pool.slice(0, questionCount);

    currentIndex = 0;
    score = 0;
    streak = 0;
    bestStreak = 0;
    totalTime = 0;

    document.getElementById("score").textContent = "0";
    document.getElementById("question-total").textContent = gameQuestions.length;

    showScreen("game");
    showQuestion();
  });

  function showQuestion() {
    answered = false;
    const q = gameQuestions[currentIndex];

    document.getElementById("question-num").textContent = currentIndex + 1;
    document.getElementById("question-text").textContent = q.question;
    document.getElementById("category-badge").textContent = q.category;

    const diffBadge = document.getElementById("difficulty-badge");
    diffBadge.textContent = q.difficulty;
    diffBadge.dataset.diff = q.difficulty;

    // Progress bar
    const pct = ((currentIndex) / gameQuestions.length) * 100;
    document.getElementById("progress-fill").style.width = pct + "%";

    // Shuffle answers
    const answers = shuffle([q.correct_answer, ...q.wrong_answers]);
    const container = document.getElementById("answers");
    container.innerHTML = "";

    answers.forEach((ans, i) => {
      const btn = document.createElement("button");
      btn.className = "answer-btn";
      btn.innerHTML = `<span class="answer-letter">${LETTERS[i]}</span><span>${ans}</span>`;
      btn.addEventListener("click", () => handleAnswer(btn, ans, q.correct_answer));
      container.appendChild(btn);
    });

    // Hide feedback
    document.getElementById("feedback").classList.add("hidden");
    document.getElementById("question-card").style.display = "block";

    startTimer();
  }

  function startTimer() {
    timeLeft = TIMER_SECONDS;
    const timerEl = document.getElementById("timer");
    timerEl.textContent = timeLeft;
    timerEl.className = "timer";

    clearInterval(timerInterval);
    timerInterval = setInterval(() => {
      timeLeft--;
      timerEl.textContent = timeLeft;

      if (timeLeft <= 5) {
        timerEl.className = "timer danger";
      } else if (timeLeft <= 10) {
        timerEl.className = "timer warning";
      }

      if (timeLeft <= 0) {
        clearInterval(timerInterval);
        handleTimeout();
      }
    }, 1000);
  }

  function handleTimeout() {
    if (answered) return;
    answered = true;
    streak = 0;

    const q = gameQuestions[currentIndex];
    const buttons = document.querySelectorAll(".answer-btn");
    buttons.forEach((btn) => {
      btn.classList.add("disabled");
      if (btn.querySelector("span:last-child").textContent === q.correct_answer) {
        btn.classList.add("correct");
      }
    });

    totalTime += TIMER_SECONDS;
    showFeedback(false, q.correct_answer, true);
  }

  function handleAnswer(btn, selected, correct) {
    if (answered) return;
    answered = true;
    clearInterval(timerInterval);

    const timeTaken = TIMER_SECONDS - timeLeft;
    totalTime += timeTaken;

    const isCorrect = selected === correct;
    const buttons = document.querySelectorAll(".answer-btn");

    buttons.forEach((b) => {
      b.classList.add("disabled");
      if (b.querySelector("span:last-child").textContent === correct) {
        b.classList.add("correct");
      }
    });

    if (isCorrect) {
      score++;
      streak++;
      if (streak > bestStreak) bestStreak = streak;
      document.getElementById("score").textContent = score;
    } else {
      btn.classList.add("wrong");
      streak = 0;
    }

    showFeedback(isCorrect, correct);
  }

  function showFeedback(isCorrect, correctAnswer, timeout = false) {
    const feedback = document.getElementById("feedback");
    feedback.classList.remove("hidden");

    document.getElementById("feedback-icon").textContent = timeout
      ? "Time's up!"
      : isCorrect
        ? "Correct!"
        : "Wrong!";

    document.getElementById("feedback-text").textContent = isCorrect
      ? `+1 point | Streak: ${streak}`
      : `The answer was: ${correctAnswer}`;

    const nextBtn = document.getElementById("btn-next");
    nextBtn.textContent =
      currentIndex < gameQuestions.length - 1 ? "Next Question" : "See Results";
  }

  // Next question
  document.getElementById("btn-next").addEventListener("click", () => {
    currentIndex++;
    if (currentIndex < gameQuestions.length) {
      showQuestion();
    } else {
      showResults();
    }
  });

  // Keyboard shortcut: 1-4 to answer, Enter/Space for next
  document.addEventListener("keydown", (e) => {
    if (!screens.game.classList.contains("active")) return;

    if (!answered) {
      const num = parseInt(e.key);
      if (num >= 1 && num <= 4) {
        const buttons = document.querySelectorAll(".answer-btn");
        if (buttons[num - 1]) buttons[num - 1].click();
      }
    } else {
      if (e.key === "Enter" || e.key === " ") {
        e.preventDefault();
        document.getElementById("btn-next").click();
      }
    }
  });

  function showResults() {
    clearInterval(timerInterval);

    const total = gameQuestions.length;
    const pct = Math.round((score / total) * 100);
    const avgTime = total > 0 ? (totalTime / total).toFixed(1) : 0;

    document.getElementById("final-score").textContent = score;
    document.getElementById("final-total").textContent = total;
    document.getElementById("score-percent").textContent = pct + "%";

    let message;
    if (pct === 100) message = "Perfect score! You're a trivia master!";
    else if (pct >= 80) message = "Excellent! You really know your stuff!";
    else if (pct >= 60) message = "Great job! Solid performance!";
    else if (pct >= 40) message = "Not bad! Keep practicing!";
    else message = "Better luck next time!";
    document.getElementById("score-message").textContent = message;

    document.getElementById("stat-correct").textContent = score;
    document.getElementById("stat-wrong").textContent = total - score;
    document.getElementById("stat-streak").textContent = bestStreak;
    document.getElementById("stat-avg-time").textContent = avgTime + "s";

    document.getElementById("progress-fill").style.width = "100%";
    showScreen("results");
  }

  // Play again (same settings)
  document.getElementById("btn-play-again").addEventListener("click", () => {
    document.getElementById("btn-start").click();
  });

  // New game (back to settings)
  document.getElementById("btn-new-game").addEventListener("click", () => {
    showScreen("start");
  });

  loadQuestions();
})();
