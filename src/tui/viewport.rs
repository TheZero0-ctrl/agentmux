pub(super) fn visible_capacity(inner_height: u16, row_height: usize) -> usize {
    if row_height == 0 {
        return 0;
    }
    usize::from(inner_height)
        .checked_div(row_height)
        .map_or(1, |capacity| capacity.max(1))
}

pub(super) fn start_index(row_count: usize, selected_index: usize, capacity: usize) -> usize {
    if row_count == 0 || capacity == 0 || selected_index < capacity {
        return 0;
    }
    selected_index
        .saturating_add(1)
        .saturating_sub(capacity)
        .min(row_count.saturating_sub(1))
}
