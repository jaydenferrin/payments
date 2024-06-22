pub mod payments
{
    use std::collections::{HashMap, HashSet};
    use regex::Regex;
    
    #[derive(Debug)]
    struct Participant
    {
        pub name: String,
        pub tasks: HashSet<String>,
        pub paid_tasks: HashSet<String>,
        pub sum: Option<f32>,
    }
    
    #[derive(Debug)]
    struct Task
    {
        pub name: String,
        pub owner: String,
        pub participants: HashSet<String>,
        pub cost: i32,
    }

    #[derive(Debug)]
    pub struct Payment
    {
        participants: HashMap<String, Participant>,
        tasks: HashMap<String, Task>,
    }

    impl Payment
    {
        pub fn new () -> Self
        {
            Self {
                participants: HashMap::new (),
                tasks: HashMap::new ()
            }
        }

        pub fn command (&mut self, com: &String) -> Result<(), &'static str>
        {
            let parts = Regex::new (r"\s")
                .unwrap ()
                .split (&com)
                .collect::<Vec<&str>> ();
            match parts.get (0)
            {
                Some (&"add")   => return self.add (parts.get (1).copied ()),
                Some (&"part")  => return self.part (parts.get (1).copied (),
                                                     parts.get (2).copied ()),
                Some (&"pay")   => return self.pay (parts.get (1).copied (),
                                                    parts.get (2).copied (),
                                                    parts.get (3).copied ()),
                Some (&"print") => self.print (),
                Some (_)        => return Err("not a command"),
                None            => return Err("command formatted incorrectly"),
            }
            Ok (())
        }

        fn calculate (&mut self)
        {
            for part in self.participants.values_mut ()
            {
                let mut sum = 0f32;
                for task_name in part.tasks.iter ()
                {
                    // divide the cost of this task among its participants and
                    // add that amount to the amount this participant owes
                    let task = self.tasks.get (task_name).unwrap ();
                    sum += (task.cost as f32
                            / (task.participants.len () as f32));
                }
                for task_name in part.paid_tasks.iter ()
                {
                    // same as before but subtracting since this participant
                    // has already paid their share of this task
                    let task = self.tasks.get (task_name).unwrap ();
                    sum -= task.cost as f32;
                }
                part.sum = Some (sum.round () / 100f32);
            }
        }

        fn print (&mut self)
        {
            self.calculate ();
            for part in self.participants.values ()
            {
                println! ("{} owes {}", part.name, part.sum.unwrap ());
            }
        }

        fn add (&mut self, try_name: Option<&str>) -> Result<(), &'static str>
        {
            // TODO if empty, err
            let name = match try_name
            {
                Some (n) => n,
                None => return Err ("Not enough arguments"),
            };
            // if there is already a participant with this name, we don't want
            // to overwrite them
            if self.participants.contains_key (name)
            {
                return Err("participant {name} was already added");
            }
            self.participants.insert (String::from (name), Participant
                              {
                                  name: String::from (name),
                                  tasks: HashSet::new (),
                                  paid_tasks: HashSet::new (),
                                  sum: None,
                              });
            Ok (())
        }

        fn pay (&mut self, try_name: Option<&str>, try_task: Option<&str>, try_price: Option<&str>)
            -> Result<(), &'static str>
        {
            let name = match try_name
            {
                Some (n) => n,
                None => return Err ("Not enough arguments"),
            };
            let task_name = match try_task
            {
                Some (n) => n,
                None => return Err ("Not enough arguments"),
            };
            let price_string = match try_price
            {
                Some (n) => n,
                None => return Err ("Not enough arguments"),
            };
            let price = match price_string.parse::<f32> ()
            {
                Ok (p) => p,
                Err (_) => return Err ("Must input a valid decimal number for the price"),
            };
            // if this participant doesn't yet exist, add them
            if !self.participants.contains_key (name)
            {
                self.participants.insert (String::from (name), Participant
                                  {
                                      name: String::from (name),
                                      tasks: HashSet::new (),
                                      paid_tasks: HashSet::new (),
                                      sum: None,
                                  });
            }
            // if there is already a task with this name, we don't want
            // to overwrite them
            if self.tasks.contains_key (task_name)
            {
                return Err ("task {task_name} was already added");
            }
            self.tasks.insert (String::from (task_name), Task
                          {
                              name: String::from (task_name),
                              owner: String::from (name),
                              participants: HashSet::new (),
                              cost: (price * 100f32) as i32,
                          });
            // add this task to the paid tasks of the participant
            let mut participant = self.participants.get_mut (name).unwrap ();
            participant.paid_tasks.insert (String::from (task_name));
            participant.tasks.insert (String::from (task_name));
            self.tasks.get_mut (task_name)
                .unwrap ()
                .participants
                .insert (String::from (name));
            Ok (())
        }

        fn part (&mut self, try_name: Option<&str>, try_task: Option<&str>) -> Result<(), &'static str>
        {
            let name = match try_name
            {
                Some (n) => n,
                None => return Err ("Not enough arguments"),
            };
            let task_name = match try_task
            {
                Some (n) => n,
                None => return Err ("Not enough arguments"),
            };
            let mut task = match self.tasks.get_mut (task_name)
            {
                Some (result) => result,
                None => return Err ("Task {task_name} has not yet been added"),
            };
            let mut participant = match self.participants.get_mut (name)
            {
                Some (result) => result,
                None => return Err ("participant {name} has not yet been added"),
            };
            // if this participant is paying for this task, don't add it to their list of tasks
            if participant.paid_tasks.contains (task_name)
            {
                return Ok (());
            }
            // done with preparing, add stuff together
            participant.tasks.insert (String::from (task_name));
            participant.sum = None;
            task.participants.insert (String::from (name));
            Ok (())
        }
    }
}
