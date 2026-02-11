use shared::dto::analytics::{AchievementCategoryDto, PlayerAchievementsDto};
use yew::prelude::*;

#[derive(Properties, PartialEq)]
pub struct AchievementsTabProps {
    pub achievements: Option<PlayerAchievementsDto>,
    pub loading: bool,
    pub error: Option<String>,
}

fn category_icon(category: &AchievementCategoryDto) -> &'static str {
    match category {
        AchievementCategoryDto::Wins => "🏆",
        AchievementCategoryDto::Contests => "🎯",
        AchievementCategoryDto::Streaks => "🔥",
        AchievementCategoryDto::Games => "🎮",
        AchievementCategoryDto::Venues => "📍",
        AchievementCategoryDto::Special => "✨",
    }
}

#[function_component(AchievementsTab)]
pub fn achievements_tab(props: &AchievementsTabProps) -> Html {
    let status_filter = use_state(|| "all".to_string());
    let category_filter = use_state(|| "all".to_string());
    let sort_by = use_state(|| "progress".to_string());

    if props.loading {
        return html! {
            <div class="bg-white rounded-xl shadow-mobile-soft p-6 border border-gray-100">
                <div class="text-center py-8 text-gray-500">{"Loading achievements..."}</div>
            </div>
        };
    }

    if let Some(error) = &props.error {
        return html! {
            <div class="bg-white rounded-xl shadow-mobile-soft p-6 border border-gray-100">
                <div class="text-center py-8 text-red-600">{error}</div>
            </div>
        };
    }

    let achievements = if let Some(data) = &props.achievements {
        data
    } else {
        return html! {
            <div class="bg-white rounded-xl shadow-mobile-soft p-6 border border-gray-100">
                <div class="text-center py-8 text-gray-500">
                    {"No achievements yet. Play more contests to unlock them!"}
                </div>
            </div>
        };
    };

    let filtered: Vec<_> = achievements
        .achievements
        .iter()
        .filter(|achievement| {
            match status_filter.as_str() {
                "unlocked" => achievement.unlocked,
                "locked" => !achievement.unlocked,
                _ => true,
            }
        })
        .filter(|achievement| {
            if category_filter.as_str() == "all" {
                true
            } else {
                achievement.category.to_string() == *category_filter
            }
        })
        .collect();

    let category_order = vec![
        AchievementCategoryDto::Wins,
        AchievementCategoryDto::Contests,
        AchievementCategoryDto::Streaks,
        AchievementCategoryDto::Games,
        AchievementCategoryDto::Venues,
        AchievementCategoryDto::Special,
    ];

    let sort_items = |items: &mut Vec<&shared::dto::analytics::AchievementDto>, sort_by: &str| {
        match sort_by {
            "name" => items.sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase())),
            "recent" => items.sort_by(|a, b| b.unlocked_at.cmp(&a.unlocked_at)),
            _ => items.sort_by(|a, b| {
                let a_req = a.required_value.max(1) as f64;
                let b_req = b.required_value.max(1) as f64;
                let a_progress = (a.current_value as f64 / a_req).min(1.0);
                let b_progress = (b.current_value as f64 / b_req).min(1.0);
                b_progress
                    .partial_cmp(&a_progress)
                    .unwrap_or(std::cmp::Ordering::Equal)
            }),
        }
    };

    let grouped: Vec<(AchievementCategoryDto, Vec<&shared::dto::analytics::AchievementDto>)> = category_order
        .iter()
        .map(|category| {
            let items = filtered
                .iter()
                .filter(|achievement| achievement.category == *category)
                .cloned()
                .collect::<Vec<_>>();
            (category.clone(), items)
        })
        .filter(|(_, items)| !items.is_empty())
        .collect();

    html! {
        <div class="space-y-6">
            <div class="bg-white rounded-xl shadow-mobile-soft p-6 border border-gray-100">
                <div class="flex items-center justify-between mb-4">
                    <div>
                        <h2 class="text-2xl font-bold text-gray-900">{"Achievements"}</h2>
                        <p class="mt-1 text-gray-600">{"Track your progress and unlock new milestones."}</p>
                    </div>
                    <div class="text-4xl">{"🏅"}</div>
                </div>
                <div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
                    <div class="bg-blue-50 rounded-lg p-4">
                        <p class="text-xs font-medium text-blue-700">{"Unlocked"}</p>
                        <p class="text-2xl font-bold text-blue-900">{achievements.unlocked_achievements}</p>
                    </div>
                    <div class="bg-gray-50 rounded-lg p-4">
                        <p class="text-xs font-medium text-gray-600">{"Total"}</p>
                        <p class="text-2xl font-bold text-gray-900">{achievements.total_achievements}</p>
                    </div>
                    <div class="bg-green-50 rounded-lg p-4">
                        <p class="text-xs font-medium text-green-700">{"Completion"}</p>
                        <p class="text-2xl font-bold text-green-900">
                            {format!("{:.0}%", achievements.completion_percentage)}
                        </p>
                    </div>
                </div>
            </div>

            <div class="bg-gray-50 rounded-lg p-4">
                <div class="grid grid-cols-1 sm:grid-cols-3 gap-4">
                    <div>
                        <label class="block text-xs font-medium text-gray-500 mb-1">{"Status"}</label>
                        <select
                            class="w-full border border-gray-200 rounded-md px-2 py-1 text-sm"
                            value={(*status_filter).clone()}
                            onchange={{
                                let status_filter = status_filter.clone();
                                Callback::from(move |event: Event| {
                                    let value = event
                                        .target_unchecked_into::<web_sys::HtmlSelectElement>()
                                        .value();
                                    status_filter.set(value);
                                })
                            }}
                        >
                            <option value="all">{"All"}</option>
                            <option value="unlocked">{"Unlocked"}</option>
                            <option value="locked">{"Locked"}</option>
                        </select>
                    </div>
                    <div>
                        <label class="block text-xs font-medium text-gray-500 mb-1">{"Category"}</label>
                        <select
                            class="w-full border border-gray-200 rounded-md px-2 py-1 text-sm"
                            value={(*category_filter).clone()}
                            onchange={{
                                let category_filter = category_filter.clone();
                                Callback::from(move |event: Event| {
                                    let value = event
                                        .target_unchecked_into::<web_sys::HtmlSelectElement>()
                                        .value();
                                    category_filter.set(value);
                                })
                            }}
                        >
                            <option value="all">{"All"}</option>
                            <option value="Wins">{"Wins"}</option>
                            <option value="Contests">{"Contests"}</option>
                            <option value="Streaks">{"Streaks"}</option>
                            <option value="Games">{"Games"}</option>
                            <option value="Venues">{"Venues"}</option>
                            <option value="Special">{"Special"}</option>
                        </select>
                    </div>
                    <div>
                        <label class="block text-xs font-medium text-gray-500 mb-1">{"Sort"}</label>
                        <select
                            class="w-full border border-gray-200 rounded-md px-2 py-1 text-sm"
                            value={(*sort_by).clone()}
                            onchange={{
                                let sort_by = sort_by.clone();
                                Callback::from(move |event: Event| {
                                    let value = event
                                        .target_unchecked_into::<web_sys::HtmlSelectElement>()
                                        .value();
                                    sort_by.set(value);
                                })
                            }}
                        >
                            <option value="progress">{"Progress"}</option>
                            <option value="recent">{"Recently unlocked"}</option>
                            <option value="name">{"Name (A-Z)"}</option>
                        </select>
                    </div>
                </div>
            </div>

            {if grouped.is_empty() {
                html! {
                    <div class="bg-white rounded-xl shadow-mobile-soft p-6 border border-gray-100">
                        <div class="text-center py-8 text-gray-500">{"No achievements match those filters."}</div>
                    </div>
                }
            } else {
                html! {
                    <div class="space-y-6">
                        {grouped.iter().map(|(category, items)| {
                            let category_label = category.to_string();
                            let icon = category_icon(category);
                            let mut sorted_items = items.clone();
                            sort_items(&mut sorted_items, &*sort_by);
                            html! {
                                <div class="bg-white rounded-xl shadow-mobile-soft p-6 border border-gray-100">
                                    <div class="flex items-center justify-between mb-4">
                                        <h3 class="text-lg font-semibold text-gray-900">
                                            {format!("{} {}", icon, category_label)}
                                        </h3>
                                        <span class="text-xs text-gray-500">
                                            {format!("{} achievements", sorted_items.len())}
                                        </span>
                                    </div>
                                    <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                                        {sorted_items.iter().map(|achievement| {
                                            let required = achievement.required_value.max(1);
                                            let progress = ((achievement.current_value as f64 / required as f64) * 100.0)
                                                .min(100.0);
                                            html! {
                                                <div class={classes!(
                                                    "rounded-lg", "border", "p-4",
                                                    if achievement.unlocked {
                                                        classes!("bg-green-50", "border-green-200")
                                                    } else {
                                                        classes!("bg-gray-50", "border-gray-200")
                                                    }
                                                )}>
                                                    <div class="flex items-start justify-between gap-4">
                                                        <div>
                                                            <h4 class="text-sm font-semibold text-gray-900">{achievement.name.clone()}</h4>
                                                            <p class="text-xs text-gray-600 mt-1">{achievement.description.clone()}</p>
                                                        </div>
                                                        {if achievement.unlocked {
                                                            html! { <span class="text-xs font-semibold text-green-700">{"Unlocked"}</span> }
                                                        } else {
                                                            html! { <span class="text-xs font-semibold text-gray-500">{"In progress"}</span> }
                                                        }}
                                                    </div>
                                                    <div class="mt-3">
                                                        <div class="flex justify-between text-xs text-gray-600 mb-1">
                                                            <span>{format!("{}/{}", achievement.current_value, achievement.required_value)}</span>
                                                            <span>{format!("{:.0}%", progress)}</span>
                                                        </div>
                                                        <div class="h-2 w-full rounded-full bg-gray-200">
                                                            <div
                                                                class={classes!(
                                                                    "h-2", "rounded-full",
                                                                    if achievement.unlocked { "bg-green-500" } else { "bg-blue-500" }
                                                                )}
                                                                style={format!("width: {:.0}%;", progress)}
                                                            ></div>
                                                        </div>
                                                        {achievement.unlocked_at.as_ref().map(|dt| html! {
                                                            <div class="text-xs text-gray-500 mt-2">
                                                                {format!("Unlocked on {}", dt)}
                                                            </div>
                                                        }).unwrap_or_else(|| html! {})}
                                                    </div>
                                                </div>
                                            }
                                        }).collect::<Html>()}
                                    </div>
                                </div>
                            }
                        }).collect::<Html>()}
                    </div>
                }
            }}
        </div>
    }
}
