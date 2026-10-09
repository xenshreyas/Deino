//! Built-ins for learning and listening: classes, webinars and podcasts.

use deino_domain::SectionFormat::{Bullets, Paragraph, Todo};

use super::{BuiltinRule, BuiltinTemplate, section};

/// A lecture or class.
pub(super) const LECTURE: BuiltinTemplate = BuiltinTemplate {
    id: "lecture",
    version: 1,
    name: "Lecture / class",
    description: "A lecture, class or workshop: key concepts, examples and study notes.",
    meeting_context: "The user is learning. Write study notes: explain concepts clearly, keep definitions, formulas and terms exact, and flag anything the teacher said will be assessed.",
    sections: &[
        section(
            "Summary",
            Paragraph,
            "What the session covered and its main takeaway.",
        ),
        section(
            "Key concepts",
            Bullets,
            "Each concept with a short definition or explanation, grouped by topic.",
        ),
        section(
            "Examples",
            Bullets,
            "Worked examples, cases and demonstrations used to explain the concepts.",
        ),
        section(
            "Assignments and deadlines",
            Todo,
            "Homework, readings and exam information, with due dates.",
        ),
        section(
            "To review",
            Bullets,
            "Points that were unclear or worth revisiting, and questions students asked.",
        ),
    ],
    rules: &[BuiltinRule::titled(
        "class",
        r"\b(lecture|class|seminar|lesson|course|tutorial)\b",
    )],
};

/// A webinar or talk.
pub(super) const WEBINAR: BuiltinTemplate = BuiltinTemplate {
    id: "webinar",
    version: 1,
    name: "Webinar",
    description: "A webinar, talk or panel: the speakers' main points, resources and Q&A.",
    meeting_context: "The user attended a presentation. Capture the speakers' arguments and data accurately, attribute them, and list the resources mentioned.",
    sections: &[
        section(
            "Overview",
            Bullets,
            "The topic, the speakers and the three to five main takeaways.",
        ),
        section(
            "Key points",
            Bullets,
            "The main arguments in order, with data and examples, attributed to speakers.",
        ),
        section(
            "Resources",
            Bullets,
            "Tools, links, books and other resources mentioned.",
        ),
        section(
            "Q&A",
            Bullets,
            "Audience questions in bold, each followed by the answer.",
        ),
    ],
    rules: &[BuiltinRule::titled(
        "webinar",
        r"\b(webinar|keynote|panel|fireside|talk)\b",
    )],
};

/// A podcast conversation.
pub(super) const PODCAST: BuiltinTemplate = BuiltinTemplate {
    id: "podcast",
    version: 1,
    name: "Podcast",
    description: "A podcast episode, recorded or listened to: topics, the guest's insights, quotes and mentions.",
    meeting_context: "A podcast conversation the user recorded or listened to. Capture the guest's ideas faithfully and the best quotes for show notes.",
    sections: &[
        section(
            "Episode summary",
            Paragraph,
            "Two or three sentences on who was on the show and what it covered.",
        ),
        section(
            "Topics",
            Bullets,
            "Each topic in the order discussed, with the key points.",
        ),
        section(
            "Notable quotes",
            Bullets,
            "Verbatim, quotable lines in quotation marks, attributed.",
        ),
        section(
            "Mentions",
            Bullets,
            "People, companies, books and links mentioned.",
        ),
    ],
    rules: &[BuiltinRule::titled("podcast", r"\b(podcast|episode)\b")],
};
