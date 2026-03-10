document.addEventListener("DOMContentLoaded", () => {
    const cmdInput = document.getElementById("cmd-input");
    const terminalScreen = document.getElementById("terminal-screen");
    const homeScreen = document.getElementById("home-screen");

    // Re-focus the terminal input if the user clicks anywhere on the terminal screen
    terminalScreen.addEventListener("click", () => {
        cmdInput.focus();
    });

    // Listen for the "Enter" key on the terminal input
    cmdInput.addEventListener("keydown", (event) => {
        if (event.key === "Enter") {
            const inputValue = cmdInput.value.trim();
            
            if (inputValue.length > 0) {
                // Hide terminal screen
                terminalScreen.classList.remove("active");
                terminalScreen.classList.add("hidden");
                
                // Show home screen
                homeScreen.classList.remove("hidden");
                homeScreen.classList.add("active");
                
                // Play a sound effect or do a transition here
            }
        }
    });

    // Prevent navigation
    const disabledLinks = document.querySelectorAll(".disable-click");
    disabledLinks.forEach(link => {
        link.addEventListener("click", (e) => {
            e.preventDefault();
            console.log("ACCESS DENIED: Insufficient clearance level.");
        });
    });

    // randomize flicker further by adjusting screen opacity on irregular intervals
    function randomFlicker() {
        const screens = document.querySelectorAll('.screen');
        screens.forEach(s => {
            const current = parseFloat(getComputedStyle(s).opacity) || 1;
            let delta = (Math.random() * 0.1) - 0.05; // +/-0.05
            let val = current + delta;
            val = Math.max(0.7, Math.min(1, val));
            s.style.opacity = val;
        });
        const delay = 300 + Math.random() * 1000; // 300ms to 1.3s
        setTimeout(randomFlicker, delay);
    }
    randomFlicker();

    // Clock update function
    const clockTime = document.querySelector("#system-clock span:first-child");
    const clockMeridian = document.querySelector(".meridian");

    function updateClock() {
        if (!clockTime || !clockMeridian) return;
        const now = new Date();
        let hours = now.getHours();
        const minutes = now.getMinutes().toString().padStart(2, '0');
        const ampm = hours >= 12 ? 'PM' : 'AM';
        
        hours = hours % 12;
        hours = hours ? hours : 12; 
        const hoursStr = hours.toString().padStart(2, '0');
        
        clockTime.textContent = `${hoursStr}:${minutes}`;
        clockMeridian.textContent = ampm;
    }
    
    updateClock();
    setInterval(updateClock, 1000);
});