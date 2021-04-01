use std::time::{SystemTime, Duration};


#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub error_threshold: u64,
    pub error_reset: Duration,
    pub retry_delay: Duration,
}

#[derive(Debug)]
pub enum CircuitBreakerState {
    Open(OpenState),
    HalfOpen(HalfOpenState),
    Close(CloseState),
}

#[derive(Debug)]
pub struct CloseState {
    pub errors: u64, 
    pub last_error: SystemTime,
}

#[derive(Debug)]
pub struct HalfOpenState {
    pub last_attempt: SystemTime,
}

#[derive(Debug)]
pub struct OpenState {
    pub last_attempt: SystemTime,
}


impl CircuitBreakerState {

    pub fn check_state(&mut self, config: &CircuitBreakerConfig) -> bool {
        let now = SystemTime::now();
        match self {
            CircuitBreakerState::Open(state) => {
                if now.duration_since(state.last_attempt).unwrap() >= config.retry_delay {
                    *self = CircuitBreakerState::HalfOpen(HalfOpenState {last_attempt: now});
                    return true;
                } else {
                    return false;
                }
            },
            CircuitBreakerState::Close(_state) => {
                return true;
            },
            CircuitBreakerState::HalfOpen(_state) => {
                return false;
            },
        }
    }

    pub fn success(&mut self, _config: &CircuitBreakerConfig) {
        let now = SystemTime::now();
        match self {
            CircuitBreakerState::Open(_state) => {
                *self = CircuitBreakerState::HalfOpen(HalfOpenState {last_attempt: now})
            },
            CircuitBreakerState::Close(_state) => {
                // pass
            },
            CircuitBreakerState::HalfOpen(_state) => {
                *self = CircuitBreakerState::Close(CloseState {
                    errors: 0,
                    last_error: now,
                })
            },
        }
    }

    pub fn error(&mut self, config: &CircuitBreakerConfig) {
        let now = SystemTime::now();
        match self {
            CircuitBreakerState::Open(_state) => {
                // pass
            },
            CircuitBreakerState::Close(state) => {
                if now.duration_since(state.last_error).unwrap() >= config.error_reset {
                    *self = CircuitBreakerState::Close(CloseState { errors: 1, last_error: now });
                } else {
                    if state.errors >= config.error_threshold {
                        *self = CircuitBreakerState::Open(OpenState {last_attempt: now})
                    } else {
                        *self = CircuitBreakerState::Close(CloseState {
                            errors: state.errors + 1,
                            last_error: now,
                        })
                    }
                }
            },
            CircuitBreakerState::HalfOpen(_state) => {
                *self = CircuitBreakerState::Open(OpenState {last_attempt: now});
            },
        }
    }
}