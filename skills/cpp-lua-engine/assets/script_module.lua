-- Reload-friendly gameplay module. State lives on self only.
local M = {}

function M.new(entity)
    return {
        entity = entity,
        persist = { "acc" },
        acc = 0,
        on_attach = M.on_attach,
        on_update = M.on_update,
        on_event = M.on_event,
        on_detach = M.on_detach,
        on_reload = M.on_reload,
    }
end

function M.on_attach(self)
end

function M.on_update(self, dt)
    -- Prefer events over per-frame update when possible.
    self.acc = self.acc + dt
end

function M.on_event(self, name, ...)
end

function M.on_detach(self)
end

function M.on_reload(self, old)
    self.acc = old.acc
end

return M
